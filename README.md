# hash-guard

A simple Rust-based CLI tool that accepts file path/glob inputs and runs or skips a bash command depending on whether the watched files have changed since the last successful run.

## How it works

`hash-guard` computes a combined SHA-256 hash over all files matched by the supplied glob patterns and compares it to a hash stored from the previous run.

- **Files changed** (or no stored state) → the command is executed.  On success the new hash is saved.
- **Files unchanged** → the command is skipped and `hash-guard` exits `0`.

When no command is supplied `hash-guard` exits `0` if files are unchanged and `1` if they have changed, so it composes naturally with shell `&&` / `||`.

## Installation

```bash
cargo install --path .
```

## Usage

```
hash-guard [OPTIONS] <GLOB>... [-- <COMMAND>...]
```

### Arguments

| Argument | Description |
|---|---|
| `<GLOB>...` | One or more file paths or glob patterns to watch |
| `-- <COMMAND>...` | Command to execute when files have changed |

### Options

| Flag | Description |
|---|---|
| `-f, --hash-file <PATH>` | Path to the hash state file (default: `.hash-guard.json`) |
| `--force` | Always run the command, even if files are unchanged |
| `-v, --verbose` | Print per-file hashes and status messages |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Examples

```bash
# Re-run cargo test only when .rs source files change
hash-guard 'src/**/*.rs' -- cargo test

# Rebuild a Docker image only when relevant files change
hash-guard Dockerfile 'src/**' -- docker build -t myapp .

# Reinstall Python dependencies only when requirements change
hash-guard requirements.txt -- pip install -r requirements.txt

# Use a custom state file (useful when running hash-guard multiple times
# in the same directory for different sets of inputs)
hash-guard -f .hg-tests.json 'tests/**' -- cargo test

# Shell composition: run a command only when files have changed
hash-guard 'src/**/*.rs' || cargo build

# Shell composition: confirm nothing has changed
hash-guard 'config/**' && echo "Config is up to date"

# Force a run regardless of file state
hash-guard --force 'src/**' -- cargo build

# Verbose output showing per-file hashes
hash-guard -v 'src/**/*.rs' -- cargo test
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Files unchanged **or** command ran successfully |
| `1` | Files changed (when no command is given) |
| `2` | hash-guard encountered an error |
| other | Exit code forwarded from the executed command |

## State file

By default hash-guard stores its state in `.hash-guard.json` in the current working directory.  You can add this file to your `.gitignore` or commit it to share the baseline state with your team.
