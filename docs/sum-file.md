---
title: The .sum file
layout: default
nav_order: 4
---

# The `.sum` file

State is stored in a plain text `.sum` file (default `.stale.sum`). By default `stale` walks up the directory tree to find the closest git repository root (a directory containing a `.git` entry — either a directory or a file, as used by worktrees and submodules) and places the file there, so you get a single `.stale.sum` per repository instead of one in every directory. The search stops at the user's home directory (`$HOME` / `%USERPROFILE%`) to avoid escaping the project tree. If no git root is found, the file is stored in the current directory. You can override this with `-f`.

The file contains one `<name> <hash>` entry per line:

```
a41dcbdfa685 e2ce01154a1476fa317b0ba5eb6b3563a3ea01e29201916212e3fef764d64c38
lint          3f4b2c9d1a8e7b6f0d5c2a1e9f8b7a6d5e4c3b2a1f0e9d8c7b6a5f4e3d2c1b0
test          7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8
```

## Name resolution

- When no `--name` is given, the name is derived from a short hash of the glob patterns and the working directory relative to the git root — the same invocation from the same directory always reuses the same entry, while different subdirectories get distinct entries to avoid collisions.
- Multiple invocations in the same directory (e.g. for lint and test) each get their own named entry in the shared `.stale.sum` file.

## Version control

You can add `.stale.sum` to `.gitignore` or commit it to share the baseline state with your team.

## Conflict markers

If `.stale.sum` is committed to version control and a merge conflict occurs, git may leave conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) inside the file.  When stale next updates the file it automatically removes those markers so the sum file returns to a clean state.

To disable this automatic cleanup — for example when you want to inspect the raw conflict before resolving — pass `--skip-cleanup`:

```bash
stale --skip-cleanup 'src/**/*.rs' -- cargo test
```

> **Note:** `--skip-cleanup` only disables the conflict-marker *stripping* step.  Stale still rewrites the sum file when it saves an updated entry (sorting all entries alphabetically), and any remaining conflict-marker lines are appended verbatim at the end of the rewritten file.  The exact layout git wrote (entries interleaved with markers) is not preserved.  If you need to resolve the conflict using git's merge tools, do so *before* running stale.
