# stale

A simple Rust-based CLI tool that accepts file path/glob inputs and runs or skips a bash command depending on whether the watched files have changed since the last successful run.

## How it works

`stale` computes a combined xxHash3 hash over all files matched by the supplied glob patterns and compares it to an entry in a `.sum` file stored from the previous run.

- **Files changed** (or no stored state) → the command is executed.  On success the new hash is saved.
- **Files unchanged** → the command is skipped and `stale` exits `0`.

When no command is supplied `stale` exits `0` if files are unchanged and `1` if they have changed, so it composes naturally with shell `&&` / `||`.

## Installation

### Shell installer (Ubuntu / Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/th1nkful/stale/main/install.sh | sh
```

Install a specific version or to a custom directory:

```bash
STALE_VERSION=0.2.0 INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/th1nkful/stale/main/install.sh | sh
```

### Homebrew (macOS/Linux)

```bash
brew tap th1nkful/stale https://github.com/th1nkful/stale
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
| `-f, --sum-file <PATH>` | Path to the `.sum` file (default: `.stale.sum` at the git root, or the current directory if not inside a git repository) |
| `-n, --name <NAME>` | Named entry in the sum file (default: short hash of the glob patterns and, when using git-root discovery, the working directory relative to the repository root) |
| `-s, --string <STRING>` | Extra string(s) to include in the hash (e.g. version numbers, environment variables) |
| `-p, --pkg <QUERY>` | Look up a package version and include it in the hash (format: `manager:package`, e.g. `npm:express`, `uv:requests`) |
| `--force` | Always run the command, even if files are unchanged |
| `-v, --verbose` | Print per-file hashes and status messages |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### Package managers (`--pkg`)

| Prefix | File parsed | Example |
|---|---|---|
| `npm` / `js` | `package.json` | `npm:express`, `js:react` |
| `uv` / `py` / `python` | `uv.lock` | `uv:requests`, `py:flask` |

Adding a new package manager requires only a new match arm and resolver function in `lib.rs`.

## The `.sum` file

State is stored in a plain text `.sum` file (default `.stale.sum`).  By
default `stale` walks up the directory tree to find the closest git
repository root (a directory containing a `.git` entry — either a directory
or a file, as used by worktrees and submodules) and places the file there,
so you get a single `.stale.sum` per repository instead of one in every
directory.  The search stops at the user's home directory (`$HOME` /
`%USERPROFILE%`) to avoid escaping the project tree.  If no git root is
found, the file is stored in the current directory.  You can override this
with `-f`.

The file contains one `<name> <hash>` entry per line:

```
a41dcbdfa685 e2ce01154a1476fa317b0ba5eb6b3563a3ea01e29201916212e3fef764d64c38
lint          3f4b2c9d1a8e7b6f0d5c2a1e9f8b7a6d5e4c3b2a1f0e9d8c7b6a5f4e3d2c1b0
test          7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8
```

- When no `--name` is given, the name is derived from a short hash of the glob patterns and the working directory relative to the git root — the same invocation from the same directory always reuses the same entry, while different subdirectories get distinct entries to avoid collisions.
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

# Re-run tests when a specific package version changes
stale -p npm:express 'src/**' -- npm test

# Re-run when a Python package is upgraded
stale -p uv:requests '*.py' -- pytest

# Multiple package versions
stale -p npm:express -p npm:react 'src/**' -- npm test

# Arbitrary version strings
stale -s "$(jq -r '.dependencies.express' package.json)" 'src/**' -- npm test

# Environment-dependent strings
stale -s "$NODE_ENV" -s "$(cat .tool-versions)" 'src/**' -- make build

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

