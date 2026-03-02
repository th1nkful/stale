# stale

A simple Rust-based CLI tool that accepts file path/glob inputs and runs or skips a bash command depending on whether the watched files have changed since the last successful run.

## How it works

`stale` computes a combined SHA-256 hash over all files matched by the supplied glob patterns and compares it to an entry in a `.sum` file stored from the previous run.

- **Files changed** (or no stored state) → the command is executed.  On success the new hash is saved.
- **Files unchanged** → the command is skipped and `stale` exits `0`.

When no command is supplied `stale` exits `0` if files are unchanged and `1` if they have changed, so it composes naturally with shell `&&` / `||`.

## Installation

### Homebrew (macOS/Linux)

```bash
brew install th1nkful/stale/stale
```

### From source

```bash
cargo install --path .
```

## Usage

```
stale [OPTIONS] <GLOB>... [-- <COMMAND>...]
```

### Arguments

| Argument | Description |
|---|---|
| `<GLOB>...` | One or more file paths or glob patterns to watch |
| `-- <COMMAND>...` | Command to execute when files have changed |

### Options

| Flag | Description |
|---|---|
| `-f, --sum-file <PATH>` | Path to the `.sum` file (default: `.stale.sum`) |
| `-n, --name <NAME>` | Named entry in the sum file (default: short hash of the glob patterns) |
| `--force` | Always run the command, even if files are unchanged |
| `-v, --verbose` | Print per-file hashes and status messages |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## The `.sum` file

State is stored in a plain text `.sum` file (default `.stale.sum`) with one `<name> <hash>` entry per line:

```
a41dcbdfa685 e2ce01154a1476fa317b0ba5eb6b3563a3ea01e29201916212e3fef764d64c38
lint          3f4b2c9d1a8e7b6f0d5c2a1e9f8b7a6d5e4c3b2a1f0e9d8c7b6a5f4e3d2c1b0
test          7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8
```

- When no `--name` is given, the name is derived from a short hash of the glob patterns — the same invocation always reuses the same entry.
- Multiple invocations in the same directory (e.g. for lint and test) each get their own named entry in the shared `.stale.sum` file.
- You can add `.stale.sum` to `.gitignore` or commit it to share the baseline state with your team.

## Examples

```bash
# Re-run cargo test only when .rs source files change
stale 'src/**/*.rs' -- cargo test

# Rebuild a Docker image only when relevant files change
stale Dockerfile 'src/**' -- docker build -t myapp .

# Track lint and test independently in the same directory
stale --name lint 'src/**/*.rs' -- cargo clippy
stale --name test 'tests/**'    -- cargo test

# Use a custom sum file
stale -f .ci.sum 'src/**' -- make build

# Shell composition: run a command only when files have changed
stale 'src/**/*.rs' || cargo build

# Shell composition: confirm nothing has changed
stale 'config/**' && echo "Config is up to date"

# Force a run regardless of file state
stale --force 'src/**' -- cargo build

# Verbose output showing per-file hashes
stale -v 'src/**/*.rs' -- cargo test
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Files unchanged **or** command ran successfully |
| `1` | Files changed (when no command is given) |
| `2` | stale encountered an error |
| other | Exit code forwarded from the executed command |

