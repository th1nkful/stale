---
title: LLMs.txt
layout: default
nav_order: 5
---

# LLMs.txt

`stale` publishes machine-readable API documentation for use with Large Language Models, generated from the Rust source by [`cargo-llms-txt`](https://github.com/masinc/cargo-llms-txt).

Two files are regenerated on every push to `main` and served as part of this site:

| File | Description |
|---|---|
| [`/llms.txt`](https://th1nkful.github.io/stale/llms.txt) | Concise project overview, dependency list, and public API summary |
| [`/llms-full.txt`](https://th1nkful.github.io/stale/llms-full.txt) | Complete public API documentation with full function signatures and doc comments |

## Example (`llms.txt`)

```
# stale

> A CLI tool to run or skip commands based on file content hashes

**Version:** 0.1.0
**License:** Apache-2.0
**Repository:** https://github.com/th1nkful/stale
**Dependencies:**
- glob (0.3)
- sha2 (0.10)
- clap (4.5) [features: derive]
- hex (0.4)
- toml (0.8)
- serde_json (1.0)
- anyhow (1.0)

Generated: 2026-03-02 19:50:11 UTC
Created by: cargo-llms-txt (https://github.com/masinc/cargo-llms-txt)

## Core Documentation

- [Complete API Documentation](llms-full.txt): Full public API documentation with detailed descriptions
- [README](README.md): Project overview and getting started guide
- [Cargo.toml](Cargo.toml): Project configuration and dependencies

## Table of Contents

### src/lib.rs

- pub fn resolve_pkg_version
- pub fn expand_globs
- pub fn compute_hash
- pub fn compute_hash_verbose
- pub fn derive_name
- pub fn find_git_root
- pub fn load_sum_entry
- pub fn save_sum_entry
```
