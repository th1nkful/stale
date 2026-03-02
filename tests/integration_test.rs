use std::fs;
use std::process::Command;

use tempfile::TempDir;

/// Return the path to the already-built `stale` binary.
///
/// In integration tests, Cargo sets `CARGO_BIN_EXE_stale` to the path of the
/// built binary, respecting the target directory and `.exe` suffix on Windows.
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_stale")
}

/// Return the path to the `test_helper` binary.
fn helper() -> &'static str {
    env!("CARGO_BIN_EXE_test_helper")
}

/// Run stale in `dir` with the given arguments, returning (stdout, stderr, exit_code).
fn run_stale(dir: &TempDir, args: &[&str]) -> (String, String, i32) {
    let bin = binary();
    let output = Command::new(&bin)
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("Failed to run stale");

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

    let (_, _, code) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code, 1, "should exit 1 (files changed, no stored state)");
}

#[test]
fn exits_0_after_state_is_saved_no_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // Save state by running with a no-op command.
    let (_, _, code) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code, 0);

    // Second check: files unchanged → exit 0.
    let (_, _, code2) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code2, 0, "should exit 0 (files unchanged)");
}

// ── command execution ────────────────────────────────────────────────────────

#[test]
fn runs_command_when_files_changed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"v1").unwrap();
    let flag_file = dir.path().join("ran.txt");

    let (_, _, code) = run_stale(
        &dir,
        &["*.txt", "--", helper(), "--touch", flag_file.to_str().unwrap()],
    );

    assert_eq!(code, 0);
    assert!(flag_file.exists(), "command should have been executed");
}

#[test]
fn skips_command_when_files_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"v1").unwrap();

    // First run saves state.
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Second run should skip.
    let counter_file = dir.path().join("count.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["*.txt", "--", helper(), "--touch", counter_file.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(!counter_file.exists(), "command should have been skipped");
}

#[test]
fn reruns_command_when_files_change() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.txt");
    fs::write(&input, b"v1").unwrap();

    // Save state for v1.
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Modify the file.
    fs::write(&input, b"v2").unwrap();

    // Should run again.
    let flag_file = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["*.txt", "--", helper(), "--touch", flag_file.to_str().unwrap()],
    );
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

    let (_, _, code) = run_stale(&dir, &["*.txt", "--", helper(), "--fail"]);
    assert_ne!(code, 0, "expected non-zero exit from failing command");

    // The .sum file should not have been created.
    let sum_file = dir.path().join(".stale.sum");
    assert!(!sum_file.exists(), "state should not be saved after failure");
}

#[test]
fn force_flag_runs_command_even_when_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"hello").unwrap();

    // Save state.
    let _ = run_stale(&dir, &["*.txt", "--", helper()]);

    let flag_file = dir.path().join("forced.txt");
    let (_, _, code) = run_stale(
        &dir,
        &["--force", "*.txt", "--", helper(), "--touch", flag_file.to_str().unwrap()],
    );
    assert_eq!(code, 0);
    assert!(flag_file.exists(), "--force should bypass the hash check");
}

#[test]
fn custom_sum_file_is_used() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"hello").unwrap();

    let custom_sum = dir.path().join("my.sum");
    let (_, _, code) = run_stale(
        &dir,
        &["-f", custom_sum.to_str().unwrap(), "*.txt", "--", helper()],
    );
    assert_eq!(code, 0);
    assert!(custom_sum.exists(), "custom sum file should be created");
}

#[test]
fn named_entries_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("input.txt"), b"hello").unwrap();

    // Save state under the name "alpha".
    let (_, _, code1) = run_stale(&dir, &["--name", "alpha", "*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // "beta" has never been run, so it should see files as changed.
    let (_, _, code2) = run_stale(&dir, &["--name", "beta", "*.txt"]);
    assert_eq!(code2, 1, "beta entry should show files as changed");

    // "alpha" should still see files as unchanged.
    let (_, _, code3) = run_stale(&dir, &["--name", "alpha", "*.txt"]);
    assert_eq!(code3, 0, "alpha entry should still be unchanged");

    // Both entries should coexist in the same .sum file.
    let sum_file = dir.path().join(".stale.sum");
    let contents = fs::read_to_string(&sum_file).unwrap();
    assert!(contents.contains("alpha "), "alpha entry missing from sum file");
}

#[test]
fn warns_when_no_files_matched() {
    let dir = tempfile::tempdir().unwrap();
    let (_, stderr, code) = run_stale(&dir, &["*.rs"]);
    assert_eq!(
        code, 1,
        "should exit 1 (effectively 'changed' with no prior state)"
    );
    assert!(
        stderr.contains("warning"),
        "should print a warning when no files match"
    );
}

