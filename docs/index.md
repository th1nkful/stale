---
title: Home
layout: home
nav_order: 1
---

# stale

A simple Rust-based CLI tool that accepts file path/glob inputs and runs or skips a command depending on whether the watched files have changed since the last successful run.
{: .fs-6 .fw-300 }

---

## How it works

`stale` computes a combined SHA-256 hash over all files matched by the supplied glob patterns and compares it to an entry in a `.sum` file stored from the previous run.

- **Files changed** (or no stored state) → the command is executed. On success the new hash is saved.
- **Files unchanged** → the command is skipped and `stale` exits `0`.

When no command is supplied `stale` exits `0` if files are unchanged and `1` if they have changed, so it composes naturally with shell `&&` / `||`.

## Quick start

```bash
# Install via Homebrew
brew install th1nkful/stale/stale

# Re-run cargo test only when .rs source files change
stale 'src/**/*.rs' -- cargo test
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Files unchanged **or** command ran successfully |
| `1` | Files changed (when no command is given) |
| `2` | stale encountered an error |
| other | Exit code forwarded from the executed command |
