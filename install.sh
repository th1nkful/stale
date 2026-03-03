#!/bin/sh
# Install script for stale — downloads a pre-built binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/th1nkful/stale/main/install.sh | sh
#
# Environment variables:
#   STALE_VERSION   — version to install (default: latest)
#   INSTALL_DIR     — directory to install to (default: /usr/local/bin)

set -eu

REPO="th1nkful/stale"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect architecture
detect_target() {
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)
      echo "Error: unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  os=$(uname -s)
  case "$os" in
    Linux)  target="${arch}-unknown-linux-gnu" ;;
    Darwin) target="${arch}-apple-darwin" ;;
    *)
      echo "Error: unsupported OS: $os" >&2
      exit 1
      ;;
  esac

  echo "$target"
}

# Resolve the version tag (latest release if STALE_VERSION is unset)
resolve_version() {
  if [ -n "${STALE_VERSION:-}" ]; then
    echo "$STALE_VERSION"
    return
  fi

  url="https://api.github.com/repos/${REPO}/releases/latest"
  if command -v curl >/dev/null 2>&1; then
    tag=$(curl -fsSL "$url" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p')
  elif command -v wget >/dev/null 2>&1; then
    tag=$(wget -qO- "$url" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p')
  else
    echo "Error: curl or wget is required" >&2
    exit 1
  fi

  if [ -z "$tag" ]; then
    echo "Error: could not determine latest version" >&2
    exit 1
  fi

  echo "$tag"
}

main() {
  target=$(detect_target)
  version=$(resolve_version)

  tarball="stale-${target}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${version}/${tarball}"

  echo "Installing stale ${version} (${target}) to ${INSTALL_DIR}..."

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "${tmpdir}/${tarball}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${tmpdir}/${tarball}" "$url"
  else
    echo "Error: curl or wget is required" >&2
    exit 1
  fi

  tar xzf "${tmpdir}/${tarball}" -C "$tmpdir"

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmpdir}/stale" "${INSTALL_DIR}/stale"
    chmod +x "${INSTALL_DIR}/stale"
  else
    sudo mv "${tmpdir}/stale" "${INSTALL_DIR}/stale"
    sudo chmod +x "${INSTALL_DIR}/stale"
  fi

  echo "stale ${version} installed to ${INSTALL_DIR}/stale"
}

main
