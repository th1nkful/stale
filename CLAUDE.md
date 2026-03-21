# CLAUDE.md

## What is stale?

A Rust CLI that runs or skips a shell command based on whether watched files have changed. It hashes file contents with xxHash3-128, stores results in a `.stale.sum` file, and compares on subsequent runs. When no command is given, exits 0 (unchanged) or 1 (changed) for shell composition.

## Build & Test Commands

```bash
cargo build                          # Build debug binary
cargo test                           # Run all unit + integration tests
cargo test --lib                     # Unit tests only (in lib.rs)
cargo test --test integration_test   # Integration tests only
cargo test <test_name>               # Run a single test by name
cargo bench                          # Run hash benchmarks (criterion)
cargo clippy                         # Lint
cargo fmt --check                    # Check formatting
```

## Architecture

**`src/lib.rs`** — All core logic as a library:
- `expand_globs()` — resolves glob patterns to sorted, deduplicated file list
- `compute_hash()` / `compute_hash_verbose()` — xxHash3-128 over file paths+contents+extra strings
- `derive_name()` — generates a deterministic 12-char entry name from glob patterns
- `load_sum_entry()` / `save_sum_entry()` — reads/writes `<name> <hash>` entries in the `.sum` file (handles conflict markers, sorting, deduplication)
- `find_git_root()` — walks up to find `.git` for sum file placement
- `resolve_pkg_version()` — looks up package versions from `package.json` (npm) or `uv.lock` (uv/python); new managers only need a new match arm + resolver function
- `find_duplicate_entries()` — detects duplicate names in sum file (from merge conflicts)

**`src/main.rs`** — CLI entry point using clap derive. Parses args, wires together lib functions, runs the subprocess, and manages exit codes (0=unchanged/success, 1=changed/command fail, 2=stale error).

**`src/bin/test_helper.rs`** — Minimal binary for integration tests (cross-platform replacement for `true`/`false`/`touch`).

**`tests/integration_test.rs`** — End-to-end tests that invoke the compiled `stale` binary in temp directories. Uses `run_stale()` helper that always passes `-f` to isolate from host git state.

## Key Conventions

- Exit codes: 0 = files unchanged or command succeeded, 1 = files changed or command failed, 2 = stale internal error
- The `.stale.sum` file defaults to the nearest git root; falls back to cwd if no git repo
- Sum file entries are always sorted alphabetically by name on write
- Git conflict markers in `.stale.sum` are auto-stripped unless `--skip-cleanup` is passed
- Hash includes file paths (not just contents), so renames count as changes
- Extra strings (`-s`) and package versions (`-p`) are mixed into the hash
- State is only persisted after a successful command execution
