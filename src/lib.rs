use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// ── package version resolution ──────────────────────────────────────────────

/// Resolve a package version query in the form `"manager:package_name"`.
///
/// Supported managers:
///
/// | Prefix        | File parsed      | Example                    |
/// |---------------|------------------|----------------------------|
/// | `npm` / `js`  | `package.json`   | `npm:express`, `js:react`  |
/// | `uv` / `py`   | `uv.lock`        | `uv:requests`, `py:flask`  |
///
/// Returns the resolved version string (e.g. `"^4.18.0"`).
///
/// Adding a new package manager only requires a new match arm and a small
/// resolver function — the rest of the pipeline is unchanged.
pub fn resolve_pkg_version(query: &str, base_dir: &Path) -> Result<String> {
    let (manager, package) = query.split_once(':').ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid package query '{query}': expected format 'manager:package' \
             (e.g. 'npm:express', 'uv:requests')"
        )
    })?;

    if package.is_empty() {
        anyhow::bail!("Package name cannot be empty in query '{query}'");
    }

    match manager {
        "npm" | "js" => resolve_npm_version(package, base_dir),
        "uv" | "py" | "python" => resolve_uv_version(package, base_dir),
        _ => Err(anyhow::anyhow!(
            "Unknown package manager '{manager}'. Supported: npm (js), uv (py/python)"
        )),
    }
}

/// Look up `package` in `package.json` under `dependencies`,
/// `devDependencies`, or `peerDependencies` (checked in that order).
fn resolve_npm_version(package: &str, base_dir: &Path) -> Result<String> {
    let pkg_path = base_dir.join("package.json");
    let contents = fs::read_to_string(&pkg_path)
        .with_context(|| format!("Failed to read {}", pkg_path.display()))?;

    let json: serde_json::Value =
        serde_json::from_str(&contents).context("Failed to parse package.json")?;

    for section in &["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(version) = json.get(section).and_then(|s| s.get(package)) {
            if let Some(v) = version.as_str() {
                return Ok(v.to_string());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Package '{package}' not found in {}",
        pkg_path.display()
    ))
}

/// Look up `package` in `uv.lock` (`[[package]]` table array).
fn resolve_uv_version(package: &str, base_dir: &Path) -> Result<String> {
    let lock_path = base_dir.join("uv.lock");
    let contents = fs::read_to_string(&lock_path)
        .with_context(|| format!("Failed to read {}", lock_path.display()))?;

    let doc: toml::Value = toml::from_str(&contents).context("Failed to parse uv.lock")?;

    if let Some(packages) = doc.get("package").and_then(|p| p.as_array()) {
        for entry in packages {
            let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if name == package {
                if let Some(version) = entry.get("version").and_then(|v| v.as_str()) {
                    return Ok(version.to_string());
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Package '{package}' not found in {}",
        lock_path.display()
    ))
}

/// Expand one or more glob patterns into a sorted, deduplicated list of file paths.
pub fn expand_globs(patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for pattern in patterns {
        let entries =
            glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {pattern}"))?;

        for entry in entries {
            let path = entry.with_context(|| format!("Error reading glob entry for {pattern}"))?;
            if path.is_file() {
                paths.push(path);
            }
        }
    }

    // Sort and deduplicate so the hash is deterministic.
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Compute a combined SHA-256 hash over the given files and optional extra
/// strings.
///
/// The hash is built by feeding each file's path and its contents into the
/// hasher in sorted order, followed by any extra strings, so the result is
/// deterministic.
pub fn compute_hash(files: &[PathBuf], extra_strings: &[String]) -> Result<String> {
    let mut hasher = Sha256::new();

    for path in files {
        // Include the path so renaming a file counts as a change.
        let path_str = path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        hasher.update(b"\0");

        let contents =
            fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        hasher.update(&contents);
        hasher.update(b"\0");
    }

    for s in extra_strings {
        hasher.update(s.as_bytes());
        hasher.update(b"\0");
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Compute a combined SHA-256 hash over the given files and optional extra
/// strings, returning a per-file breakdown alongside the combined hash.
pub fn compute_hash_verbose(
    files: &[PathBuf],
    extra_strings: &[String],
) -> Result<(String, BTreeMap<String, String>)> {
    let mut hasher = Sha256::new();
    let mut per_file: BTreeMap<String, String> = BTreeMap::new();

    for path in files {
        let path_str = path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        hasher.update(b"\0");

        let contents =
            fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;

        let mut file_hasher = Sha256::new();
        file_hasher.update(&contents);
        let file_hash = hex::encode(file_hasher.finalize());
        per_file.insert(path_str.into_owned(), file_hash);

        hasher.update(&contents);
        hasher.update(b"\0");
    }

    for s in extra_strings {
        hasher.update(s.as_bytes());
        hasher.update(b"\0");
    }

    Ok((hex::encode(hasher.finalize()), per_file))
}

/// Derive a stable short name from a list of glob patterns and optional extra
/// strings.
///
/// The name is the first 12 hex characters of the SHA-256 hash of the
/// sorted, newline-joined patterns followed by any extra strings.  This gives
/// a deterministic identifier so repeated invocations with the same patterns
/// and strings always map to the same entry in the `.sum` file without
/// requiring the user to supply `--name`.
pub fn derive_name(patterns: &[String], extra_strings: &[String]) -> String {
    let mut hasher = Sha256::new();
    let mut sorted = patterns.to_vec();
    sorted.sort();
    for p in &sorted {
        hasher.update(p.as_bytes());
        hasher.update(b"\n");
    }
    for s in extra_strings {
        hasher.update(s.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())[..12].to_string()
}

/// Look up the hash stored for `name` in the `.sum` file at `path`.
///
/// The file format is one `<name> <hash>` pair per line; lines starting with
/// `#` are treated as comments.  Returns `None` if the file does not exist or
/// the name is not present.
pub fn load_sum_entry(path: &Path, name: &str) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read sum file {}", path.display()))?;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(entry_name), Some(entry_hash)) = (parts.next(), parts.next()) {
            if entry_name == name {
                return Ok(Some(entry_hash.to_string()));
            }
        }
    }

    Ok(None)
}

/// Write (or update) the `name` entry in the `.sum` file at `path`.
///
/// The file is always rewritten with all entries sorted by name so the output
/// is stable and deterministic regardless of insertion order.
pub fn save_sum_entry(path: &Path, name: &str, hash: &str) -> Result<()> {
    // Collect existing entries, skipping comments and blank lines.
    let mut entries: Vec<(String, String)> = if path.exists() {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read sum file {}", path.display()))?;
        contents
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    None
                } else {
                    let mut parts = trimmed.split_whitespace();
                    match (parts.next(), parts.next()) {
                        (Some(n), Some(h)) => Some((n.to_string(), h.to_string())),
                        _ => None,
                    }
                }
            })
            .filter(|(n, _)| n != name) // remove the entry we're about to upsert
            .collect()
    } else {
        Vec::new()
    };

    entries.push((name.to_string(), hash.to_string()));
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let contents: String = entries
        .iter()
        .map(|(n, h)| format!("{n} {h}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    fs::write(path, contents)
        .with_context(|| format!("Failed to write sum file {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f
    }

    #[test]
    fn test_compute_hash_empty_list() {
        let hash = compute_hash(&[], &[]).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let f1 = write_temp(b"hello");
        let f2 = write_temp(b"world");
        let files = vec![f1.path().to_path_buf(), f2.path().to_path_buf()];
        let h1 = compute_hash(&files, &[]).unwrap();
        let h2 = compute_hash(&files, &[]).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_hash_changes_on_content_change() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"v1").unwrap();
        let files = vec![f.path().to_path_buf()];
        let h1 = compute_hash(&files, &[]).unwrap();

        f.reopen().unwrap();
        fs::write(f.path(), b"v2").unwrap();
        let h2 = compute_hash(&files, &[]).unwrap();

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_load_sum_entry_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-state.sum");
        let result = load_sum_entry(&missing, "myname").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_sum_entry_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let sum_path = dir.path().join("state.sum");
        save_sum_entry(&sum_path, "myname", "deadbeef").unwrap();
        let loaded = load_sum_entry(&sum_path, "myname").unwrap().unwrap();
        assert_eq!(loaded, "deadbeef");
    }

    #[test]
    fn test_save_sum_entry_updates_existing() {
        let dir = tempfile::tempdir().unwrap();
        let sum_path = dir.path().join("state.sum");
        save_sum_entry(&sum_path, "myname", "aaa").unwrap();
        save_sum_entry(&sum_path, "myname", "bbb").unwrap();
        let loaded = load_sum_entry(&sum_path, "myname").unwrap().unwrap();
        assert_eq!(loaded, "bbb");
        // Only one entry should exist for this name.
        let contents = fs::read_to_string(&sum_path).unwrap();
        assert_eq!(contents.lines().count(), 1);
    }

    #[test]
    fn test_sum_file_is_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let sum_path = dir.path().join("state.sum");
        // Insert in reverse alphabetical order.
        save_sum_entry(&sum_path, "zebra", "hash-z").unwrap();
        save_sum_entry(&sum_path, "mango", "hash-m").unwrap();
        save_sum_entry(&sum_path, "apple", "hash-a").unwrap();
        let contents = fs::read_to_string(&sum_path).unwrap();
        let names: Vec<&str> = contents
            .lines()
            .filter_map(|l| l.split_once(' ').map(|(n, _)| n))
            .collect();
        assert_eq!(names, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn test_sum_file_multiple_entries() {
        let dir = tempfile::tempdir().unwrap();
        let sum_path = dir.path().join("state.sum");
        save_sum_entry(&sum_path, "alpha", "hash-a").unwrap();
        save_sum_entry(&sum_path, "beta", "hash-b").unwrap();
        assert_eq!(
            load_sum_entry(&sum_path, "alpha").unwrap().unwrap(),
            "hash-a"
        );
        assert_eq!(
            load_sum_entry(&sum_path, "beta").unwrap().unwrap(),
            "hash-b"
        );
    }

    #[test]
    fn test_load_sum_entry_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let sum_path = dir.path().join("state.sum");
        save_sum_entry(&sum_path, "other", "hash-x").unwrap();
        let result = load_sum_entry(&sum_path, "missing").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_derive_name_stable() {
        let patterns = vec!["src/**/*.rs".to_string(), "tests/**".to_string()];
        let n1 = derive_name(&patterns, &[]);
        let n2 = derive_name(&patterns, &[]);
        assert_eq!(n1, n2);
        assert_eq!(n1.len(), 12);
    }

    #[test]
    fn test_derive_name_order_independent() {
        let a = derive_name(&["src/**".to_string(), "tests/**".to_string()], &[]);
        let b = derive_name(&["tests/**".to_string(), "src/**".to_string()], &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_expand_globs_matches_files() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");
        fs::write(&file_a, b"a").unwrap();
        fs::write(&file_b, b"b").unwrap();

        let pattern = format!("{}/*.txt", dir.path().display());
        let files = expand_globs(&[pattern]).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_expand_globs_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("a.txt");
        fs::write(&file_a, b"a").unwrap();

        let pattern = format!("{}/*.txt", dir.path().display());
        let files = expand_globs(&[pattern.clone(), pattern]).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_compute_hash_with_extra_strings() {
        let f = write_temp(b"hello");
        let files = vec![f.path().to_path_buf()];
        let h1 = compute_hash(&files, &[]).unwrap();
        let h2 = compute_hash(&files, &["extra".to_string()]).unwrap();
        assert_ne!(h1, h2, "extra strings should change the hash");
    }

    #[test]
    fn test_compute_hash_extra_strings_deterministic() {
        let f = write_temp(b"hello");
        let files = vec![f.path().to_path_buf()];
        let strings = vec!["v1.0.0".to_string()];
        let h1 = compute_hash(&files, &strings).unwrap();
        let h2 = compute_hash(&files, &strings).unwrap();
        assert_eq!(h1, h2, "same strings should produce the same hash");
    }

    #[test]
    fn test_compute_hash_different_strings_different_hash() {
        let f = write_temp(b"hello");
        let files = vec![f.path().to_path_buf()];
        let h1 = compute_hash(&files, &["v1".to_string()]).unwrap();
        let h2 = compute_hash(&files, &["v2".to_string()]).unwrap();
        assert_ne!(h1, h2, "different strings should produce different hashes");
    }

    #[test]
    fn test_compute_hash_verbose_with_extra_strings() {
        let f = write_temp(b"hello");
        let files = vec![f.path().to_path_buf()];
        let (h1, _) = compute_hash_verbose(&files, &[]).unwrap();
        let (h2, _) = compute_hash_verbose(&files, &["extra".to_string()]).unwrap();
        assert_ne!(h1, h2, "extra strings should change the verbose hash");
    }

    #[test]
    fn test_derive_name_with_strings() {
        let patterns = vec!["src/**".to_string()];
        let n1 = derive_name(&patterns, &[]);
        let n2 = derive_name(&patterns, &["v1.0.0".to_string()]);
        assert_ne!(n1, n2, "extra strings should change the derived name");
        assert_eq!(n2.len(), 12);
    }

    #[test]
    fn test_compute_hash_strings_only() {
        let h1 = compute_hash(&[], &["version=1.0".to_string()]).unwrap();
        let h2 = compute_hash(&[], &["version=2.0".to_string()]).unwrap();
        assert_ne!(h1, h2, "different strings-only hashes should differ");
        assert_eq!(h1.len(), 64);
    }

    // ── package version resolution ──────────────────────────────────────────

    #[test]
    fn test_resolve_npm_version_from_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"express":"^4.18.0"}}"#,
        )
        .unwrap();
        let v = resolve_pkg_version("npm:express", dir.path()).unwrap();
        assert_eq!(v, "^4.18.0");
    }

    #[test]
    fn test_resolve_npm_version_from_dev_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"devDependencies":{"jest":"^29.0.0"}}"#,
        )
        .unwrap();
        let v = resolve_pkg_version("js:jest", dir.path()).unwrap();
        assert_eq!(v, "^29.0.0");
    }

    #[test]
    fn test_resolve_npm_version_not_found() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"express":"^4.18.0"}}"#,
        )
        .unwrap();
        let err = resolve_pkg_version("npm:missing", dir.path());
        assert!(err.is_err());
    }

    #[test]
    fn test_resolve_uv_version() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("uv.lock"),
            r#"
version = 1

[[package]]
name = "flask"
version = "3.0.0"

[[package]]
name = "requests"
version = "2.31.0"
"#,
        )
        .unwrap();
        let v = resolve_pkg_version("uv:requests", dir.path()).unwrap();
        assert_eq!(v, "2.31.0");
    }

    #[test]
    fn test_resolve_uv_version_py_alias() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("uv.lock"),
            r#"
version = 1

[[package]]
name = "flask"
version = "3.0.0"
"#,
        )
        .unwrap();
        let v = resolve_pkg_version("py:flask", dir.path()).unwrap();
        assert_eq!(v, "3.0.0");
    }

    #[test]
    fn test_resolve_uv_version_not_found() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("uv.lock"),
            r#"
version = 1

[[package]]
name = "flask"
version = "3.0.0"
"#,
        )
        .unwrap();
        let err = resolve_pkg_version("uv:missing", dir.path());
        assert!(err.is_err());
    }

    #[test]
    fn test_resolve_unknown_manager() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_pkg_version("cargo:serde", dir.path());
        assert!(err.is_err());
    }

    #[test]
    fn test_resolve_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_pkg_version("no-colon", dir.path());
        assert!(err.is_err());
    }
}
