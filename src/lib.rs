use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Compute a combined SHA-256 hash over the given files.
///
/// The hash is built by feeding each file's path and its contents into the
/// hasher in sorted order, so the result is deterministic.
pub fn compute_hash(files: &[PathBuf]) -> Result<String> {
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

    Ok(hex::encode(hasher.finalize()))
}

/// Compute a combined SHA-256 hash over the given files, returning a per-file
/// breakdown alongside the combined hash.
pub fn compute_hash_verbose(files: &[PathBuf]) -> Result<(String, BTreeMap<String, String>)> {
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

    Ok((hex::encode(hasher.finalize()), per_file))
}

/// Derive a stable short name from a list of glob patterns.
///
/// The name is the first 12 hex characters of the SHA-256 hash of the
/// sorted, newline-joined patterns.  This gives a deterministic identifier
/// so repeated invocations with the same patterns always map to the same
/// entry in the `.sum` file without requiring the user to supply `--name`.
pub fn derive_name(patterns: &[String]) -> String {
    let mut hasher = Sha256::new();
    let mut sorted = patterns.to_vec();
    sorted.sort();
    for p in &sorted {
        hasher.update(p.as_bytes());
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
        if let Some((entry_name, entry_hash)) = line.split_once(' ') {
            if entry_name == name {
                return Ok(Some(entry_hash.trim().to_string()));
            }
        }
    }

    Ok(None)
}

/// Write (or update) the `name` entry in the `.sum` file at `path`.
///
/// If the file already contains an entry for `name` it is replaced in-place;
/// otherwise a new line is appended.
pub fn save_sum_entry(path: &Path, name: &str, hash: &str) -> Result<()> {
    let new_line = format!("{name} {hash}");

    if path.exists() {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read sum file {}", path.display()))?;

        let mut replaced = false;
        let updated: String = contents
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && trimmed.split_once(' ').map(|(n, _)| n) == Some(name)
                {
                    replaced = true;
                    new_line.clone()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        if replaced {
            // Preserve a trailing newline if the original had one.
            let trailing = if contents.ends_with('\n') { "\n" } else { "" };
            fs::write(path, format!("{updated}{trailing}"))
                .with_context(|| format!("Failed to write sum file {}", path.display()))?;
            return Ok(());
        }
    }

    // Append (or create) the entry.
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open sum file {}", path.display()))?;
    writeln!(file, "{new_line}")
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
        let hash = compute_hash(&[]).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let f1 = write_temp(b"hello");
        let f2 = write_temp(b"world");
        let files = vec![f1.path().to_path_buf(), f2.path().to_path_buf()];
        let h1 = compute_hash(&files).unwrap();
        let h2 = compute_hash(&files).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_hash_changes_on_content_change() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"v1").unwrap();
        let files = vec![f.path().to_path_buf()];
        let h1 = compute_hash(&files).unwrap();

        f.reopen().unwrap();
        fs::write(f.path(), b"v2").unwrap();
        let h2 = compute_hash(&files).unwrap();

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
        let n1 = derive_name(&patterns);
        let n2 = derive_name(&patterns);
        assert_eq!(n1, n2);
        assert_eq!(n1.len(), 12);
    }

    #[test]
    fn test_derive_name_order_independent() {
        let a = derive_name(&["src/**".to_string(), "tests/**".to_string()]);
        let b = derive_name(&["tests/**".to_string(), "src/**".to_string()]);
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
}

