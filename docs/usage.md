---
title: Usage
layout: default
nav_order: 3
---

# Usage

```
stale [OPTIONS] <GLOB>... [-- <COMMAND>...]
```

## Arguments

| Argument | Description |
|---|---|
| `<GLOB>...` | One or more file paths or glob patterns to watch |
| `-- <COMMAND>...` | Command to execute when files have changed |

## Options

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

## Package managers (`--pkg`)

| Prefix | File parsed | Example |
|---|---|---|
| `npm` / `js` | `package.json` | `npm:express`, `js:react` |
| `uv` / `py` / `python` | `uv.lock` | `uv:requests`, `py:flask` |

Adding a new package manager requires only a new match arm and resolver function in `lib.rs`.

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

# Shell composition: run a command when files have changed (also runs on errors)
stale 'src/**/*.rs' || cargo build

# Shell composition: confirm nothing has changed
stale 'config/**' && echo "Config is up to date"

# Force a run regardless of file state
stale --force 'src/**' -- cargo build

# Verbose output showing per-file hashes
stale -v 'src/**/*.rs' -- cargo test
```
