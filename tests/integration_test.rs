use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// Build a debug binary once and return its path.
fn binary() -> String {
    let output = Command::new("cargo")
        .args(["build", "--bin", "hash-guard"])
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        panic!(
            "cargo build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Locate the binary relative to CARGO_MANIFEST_DIR.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/target/debug/hash-guard")
}

/// Run hash-guard in `dir` with the given arguments, returning (stdout, stderr, exit_code).
fn run_hg(dir: &TempDir, args: &[&str]) -> (String, String, i32) {
    let bin = binary();
    let output = Command::new(&bin)
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("Failed to run hash-guard");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

// ── basic exit-code behaviour ────────────────────────────────────────────────

#[test]
fn exits_1_when_files_changed_no_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    let (_, _, code) = run_hg(&dir, &[&pattern]);
    assert_eq!(code, 1, "should exit 1 (files changed, no stored state)");
}

#[test]
fn exits_0_after_state_is_saved_no_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // Save state by running with a no-op command.
    let pattern = format!("{}/*.txt", dir.path().display());
    let (_, _, code) = run_hg(&dir, &[&pattern, "--", "true"]);
    assert_eq!(code, 0);

    // Second check: files unchanged → exit 0.
    let (_, _, code2) = run_hg(&dir, &[&pattern]);
    assert_eq!(code2, 0, "should exit 0 (files unchanged)");
}

// ── command execution ────────────────────────────────────────────────────────

#[test]
fn runs_command_when_files_changed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"v1").unwrap();
    let flag_file = dir.path().join("ran.txt");

    let pattern = format!("{}/*.txt", dir.path().display());
    let touch_cmd = format!("touch {}", flag_file.display());
    let (_, _, code) = run_hg(&dir, &[&pattern, "--", "sh", "-c", &touch_cmd]);

    assert_eq!(code, 0);
    assert!(flag_file.exists(), "command should have been executed");
}

#[test]
fn skips_command_when_files_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"v1").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    // First run saves state.
    let (_, _, code1) = run_hg(&dir, &[&pattern, "--", "true"]);
    assert_eq!(code1, 0);

    // Second run should skip.
    let counter_file = dir.path().join("count.txt");
    let touch_cmd = format!("touch {}", counter_file.display());
    let (_, _, code2) = run_hg(&dir, &[&pattern, "--", "sh", "-c", &touch_cmd]);
    assert_eq!(code2, 0);
    assert!(!counter_file.exists(), "command should have been skipped");
}

#[test]
fn reruns_command_when_files_change() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.txt");
    fs::write(&input, b"v1").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    // Save state for v1.
    let (_, _, code1) = run_hg(&dir, &[&pattern, "--", "true"]);
    assert_eq!(code1, 0);

    // Modify the file.
    fs::write(&input, b"v2").unwrap();

    // Should run again.
    let flag_file = dir.path().join("ran.txt");
    let touch_cmd = format!("touch {}", flag_file.display());
    let (_, _, code2) = run_hg(&dir, &[&pattern, "--", "sh", "-c", &touch_cmd]);
    assert_eq!(code2, 0);
    assert!(
        flag_file.exists(),
        "command should have re-executed after file change"
    );
}

#[test]
fn does_not_save_state_when_command_fails() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"v1").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    // Run a failing command.
    let (_, _, code) = run_hg(&dir, &[&pattern, "--", "false"]);
    assert_ne!(code, 0, "expected non-zero exit from failing command");

    // The state file should not have been created.
    let state = dir.path().join(".hash-guard.json");
    assert!(!state.exists(), "state should not be saved after failure");
}

#[test]
fn force_flag_runs_command_even_when_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"hello").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    // Save state.
    let (_, _, _) = run_hg(&dir, &[&pattern, "--", "true"]);

    let flag_file = dir.path().join("forced.txt");
    let touch_cmd = format!("touch {}", flag_file.display());
    let (_, _, code) = run_hg(&dir, &["--force", &pattern, "--", "sh", "-c", &touch_cmd]);
    assert_eq!(code, 0);
    assert!(flag_file.exists(), "--force should bypass the hash check");
}

#[test]
fn custom_hash_file_is_used() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"hello").unwrap();

    let pattern = format!("{}/*.txt", dir.path().display());
    let custom_state = dir.path().join("my-state.json");
    let (_, _, code) = run_hg(
        &dir,
        &["-f", custom_state.to_str().unwrap(), &pattern, "--", "true"],
    );
    assert_eq!(code, 0);
    assert!(custom_state.exists(), "custom state file should be created");
}

#[test]
fn warns_when_no_files_matched() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = format!("{}/*.rs", dir.path().display());
    let (_, stderr, code) = run_hg(&dir, &[&pattern]);
    assert_eq!(
        code, 1,
        "should exit 1 (effectively 'changed' with no prior state)"
    );
    assert!(
        stderr.contains("warning"),
        "should print a warning when no files match"
    );
}
