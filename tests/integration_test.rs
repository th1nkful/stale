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

// ── exact file paths ─────────────────────────────────────────────────────────

#[test]
fn exact_file_path_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("exact.txt");
    fs::write(&file, b"content").unwrap();

    let (_, _, code) = run_stale(&dir, &[file.to_str().unwrap(), "--", helper()]);
    assert_eq!(code, 0, "exact file path should be accepted");
}

#[test]
fn exact_file_unchanged_skips_command() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("exact.txt");
    fs::write(&file, b"content").unwrap();

    // First run saves state.
    let (_, _, code1) = run_stale(&dir, &[file.to_str().unwrap(), "--", helper()]);
    assert_eq!(code1, 0);

    // Second run with same file should skip.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &[file.to_str().unwrap(), "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(!flag.exists(), "command should be skipped for unchanged exact file");
}

#[test]
fn multiple_exact_files_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let f1 = dir.path().join("a.txt");
    let f2 = dir.path().join("b.txt");
    fs::write(&f1, b"aaa").unwrap();
    fs::write(&f2, b"bbb").unwrap();

    let (_, _, code) = run_stale(
        &dir,
        &[f1.to_str().unwrap(), f2.to_str().unwrap(), "--", helper()],
    );
    assert_eq!(code, 0, "multiple exact files should be accepted");
}

// ── specific package version strings ─────────────────────────────────────────

#[test]
fn package_version_string_triggers_rerun() {
    // Simulates: stale -s "express@4.18.0" 'src/**' -- npm test
    // When the express version changes, the hash changes and the command re-runs.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.js"), b"console.log('hello')").unwrap();

    // First run with express@4.18.0.
    let (_, _, code1) = run_stale(
        &dir,
        &["-s", "express@4.18.0", "--name", "pkg", "*.js", "--", helper()],
    );
    assert_eq!(code1, 0);

    // Same version, same source → skip.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["-s", "express@4.18.0", "--name", "pkg", "*.js", "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(!flag.exists(), "same package version should skip command");

    // Bump express version → re-run.
    let flag2 = dir.path().join("ran2.txt");
    let (_, _, code3) = run_stale(
        &dir,
        &["-s", "express@4.19.0", "--name", "pkg", "*.js", "--", helper(), "--touch", flag2.to_str().unwrap()],
    );
    assert_eq!(code3, 0);
    assert!(flag2.exists(), "bumped package version should trigger re-run");
}

#[test]
fn multiple_package_versions_trigger_rerun() {
    // Simulates passing multiple specific package versions:
    //   stale -s "requests==2.31.0" -s "flask==3.0.0" '*.py' -- pytest
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.py"), b"import flask").unwrap();

    // First run with two package versions.
    let (_, _, code1) = run_stale(
        &dir,
        &["-s", "requests==2.31.0", "-s", "flask==3.0.0", "--name", "deps", "*.py", "--", helper()],
    );
    assert_eq!(code1, 0);

    // Same versions → skip.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["-s", "requests==2.31.0", "-s", "flask==3.0.0", "--name", "deps", "*.py", "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(!flag.exists(), "same package versions should skip");

    // Bump one package → re-run.
    let flag2 = dir.path().join("ran2.txt");
    let (_, _, code3) = run_stale(
        &dir,
        &["-s", "requests==2.32.0", "-s", "flask==3.0.0", "--name", "deps", "*.py", "--", helper(), "--touch", flag2.to_str().unwrap()],
    );
    assert_eq!(code3, 0);
    assert!(flag2.exists(), "bumping one package version should trigger re-run");
}

#[test]
fn package_version_with_unchanged_source_still_reruns() {
    // Even if source files haven't changed, a version string change should re-run.
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("main.py"), b"print('hi')").unwrap();

    let (_, _, code1) = run_stale(
        &dir,
        &["-s", "numpy==1.26.0", "--name", "ver", "*.py", "--", helper()],
    );
    assert_eq!(code1, 0);

    // Source unchanged, but version bumped.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["-s", "numpy==1.27.0", "--name", "ver", "*.py", "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(flag.exists(), "version bump alone (unchanged source) should trigger re-run");
}

// ── recursive glob patterns ─────────────────────────────────────────────────

#[test]
fn recursive_glob_matches_nested_files() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub").join("deep");
    fs::create_dir_all(&sub).unwrap();
    fs::write(dir.path().join("root.txt"), b"root").unwrap();
    fs::write(sub.join("nested.txt"), b"nested").unwrap();

    let pattern = format!("{}/**/*.txt", dir.path().display());
    let (_, _, code) = run_stale(&dir, &[&pattern, "--", helper()]);
    assert_eq!(code, 0, "recursive glob should match nested files");
}

#[test]
fn recursive_glob_detects_nested_change() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("file.txt"), b"v1").unwrap();

    let pattern = format!("{}/**/*.txt", dir.path().display());

    // First run saves state.
    let (_, _, code1) = run_stale(&dir, &[&pattern, "--", helper()]);
    assert_eq!(code1, 0);

    // Modify nested file.
    fs::write(sub.join("file.txt"), b"v2").unwrap();

    // Should detect the change.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &[&pattern, "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(flag.exists(), "should detect change in nested file via recursive glob");
}

// ── string flag ──────────────────────────────────────────────────────────────

#[test]
fn string_flag_changes_hash() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // Save state with string v1.
    let (_, _, code1) = run_stale(&dir, &["-s", "v1", "*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Same file but different string should trigger re-run.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["-s", "v2", "*.txt", "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(flag.exists(), "different --string should trigger re-run");
}

#[test]
fn same_string_skips_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // Save state with string v1.
    let (_, _, code1) = run_stale(&dir, &["-s", "v1", "--name", "str-test", "*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Same string and same file should skip.
    let flag = dir.path().join("ran.txt");
    let (_, _, code2) = run_stale(
        &dir,
        &["-s", "v1", "--name", "str-test", "*.txt", "--", helper(), "--touch", flag.to_str().unwrap()],
    );
    assert_eq!(code2, 0);
    assert!(!flag.exists(), "same --string value should skip command");
}

#[test]
fn multiple_strings_accepted() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    let (_, _, code) = run_stale(
        &dir,
        &["-s", "str1", "-s", "str2", "*.txt", "--", helper()],
    );
    assert_eq!(code, 0, "multiple --string values should be accepted");
}

// ── chaining (skip / run) ───────────────────────────────────────────────────

#[test]
fn chain_skip_exits_0_when_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // First run to save state (with command).
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Check-only (no command) should exit 0 when unchanged.
    let (_, _, code2) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code2, 0, "should exit 0 when files unchanged (chain skip)");
}

#[test]
fn chain_run_exits_1_when_changed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"hello").unwrap();

    // No prior state → exit 1.
    let (_, _, code) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code, 1, "should exit 1 when files changed (chain run)");
}

#[test]
fn chain_run_exits_1_after_modification() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    fs::write(&file, b"v1").unwrap();

    // Save state.
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Modify file.
    fs::write(&file, b"v2").unwrap();

    // Should exit 1 (changed).
    let (_, _, code2) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code2, 1, "should exit 1 after file modification (chain run)");
}

// ── should-run check ────────────────────────────────────────────────────────

#[test]
fn should_run_returns_1_for_new_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("new.txt"), b"new").unwrap();

    let (_, _, code) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code, 1, "new files with no prior state should return 1");
}

#[test]
fn should_run_returns_0_after_successful_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"content").unwrap();

    // Run command to save state.
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper()]);
    assert_eq!(code1, 0);

    // Check should return 0 (unchanged).
    let (_, _, code2) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code2, 0, "should return 0 after successful command");
}

#[test]
fn should_run_returns_1_after_failed_command() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), b"content").unwrap();

    // Run failing command (state not saved).
    let (_, _, code1) = run_stale(&dir, &["*.txt", "--", helper(), "--fail"]);
    assert_ne!(code1, 0);

    // Check should return 1 (no saved state).
    let (_, _, code2) = run_stale(&dir, &["*.txt"]);
    assert_eq!(code2, 1, "should return 1 after failed command (state not saved)");
}

