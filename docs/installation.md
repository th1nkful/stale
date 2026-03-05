---
title: Installation
layout: default
nav_order: 2
---

# Installation

## Shell installer (Ubuntu / Linux / macOS)

The install script downloads a pre-built binary from GitHub Releases and places
it in `/usr/local/bin` (or a directory of your choice). This is the recommended
approach for CI/CD pipelines.

```bash
curl -fsSL https://raw.githubusercontent.com/th1nkful/stale/main/install.sh | sh
```

### Options

| Variable | Description | Default |
|---|---|---|
| `STALE_VERSION` | Version to install | latest release |
| `INSTALL_DIR` | Directory to install the binary to | `/usr/local/bin` |

```bash
# Install a specific version to a custom directory
STALE_VERSION=0.2.1 INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/th1nkful/stale/main/install.sh | sh
```

## Homebrew (macOS/Linux)

```bash
brew install th1nkful/stale/stale
```

## From source

```bash
cargo install --path .
```

## From GitHub releases

Pre-built binaries are available for the following platforms on the [GitHub Releases](https://github.com/th1nkful/stale/releases) page:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
