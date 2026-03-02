use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Stored state for a previous hash-guard run.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HashState {
    /// Combined SHA-256 hash (hex) of all matched files.
    pub hash: String,
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

/// Compute a combined SHA-256 hash over the given files.
///
/// The hash is built by feeding each file's canonicalized path and its contents
/// into the hasher in sorted order, so the result is deterministic.
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

/// Load a [`HashState`] from a JSON file.  Returns `None` if the file does not exist.
pub fn load_state(path: &Path) -> Result<Option<HashState>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read state file {}", path.display()))?;
    let state: HashState = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse state file {}", path.display()))?;
    Ok(Some(state))
}

/// Persist a [`HashState`] to a JSON file.
pub fn save_state(path: &Path, state: &HashState) -> Result<()> {
    let contents = serde_json::to_string_pretty(state).context("Failed to serialize state")?;
    fs::write(path, contents)
        .with_context(|| format!("Failed to write state file {}", path.display()))?;
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
        // SHA-256 of no input is the hash of an empty message.
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

        // Overwrite the file with different content.
        f.reopen().unwrap();
        fs::write(f.path(), b"v2").unwrap();
        let h2 = compute_hash(&files).unwrap();

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_load_state_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-state.json");
        let result = load_state(&missing).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_state_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let state = HashState {
            hash: "deadbeef".to_string(),
        };
        save_state(&state_path, &state).unwrap();
        let loaded = load_state(&state_path).unwrap().unwrap();
        assert_eq!(loaded, state);
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
        // Pass the same pattern twice.
        let files = expand_globs(&[pattern.clone(), pattern]).unwrap();
        assert_eq!(files.len(), 1);
    }
}
