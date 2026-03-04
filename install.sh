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

  # Use the /releases/latest redirect to discover the latest tag.
  # This is not subject to the same API rate limits as the REST API.
  latest_url="https://github.com/${REPO}/releases/latest"

  if command -v curl >/dev/null 2>&1; then
    # Follow redirects and capture the final URL, which ends with /tag/<tag>
    final_url=$(curl -fsSL -o /dev/null -w '%{url_effective}' "$latest_url")
    tag=$(printf '%s\n' "$final_url" | sed 's#.*/tag/##')
  elif command -v wget >/dev/null 2>&1; then
    # Inspect the Location header to find the target URL.
    final_url=$(wget -qO- --max-redirect=0 --server-response "$latest_url" 2>&1 | awk '/^  Location: / {print $2}' | tail -n 1)
    tag=$(printf '%s\n' "$final_url" | sed 's#.*/tag/##')
  else
    echo "Error: curl or wget is required" >&2
    exit 1
  fi

  if [ -z "${tag:-}" ]; then
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

  if [ ! -f "${tmpdir}/stale" ]; then
    echo "Error: expected '${tmpdir}/stale' in the extracted tarball, but it was not found." >&2
    echo "The release archive format may have changed, or the download/extraction may have failed." >&2
    exit 1
  fi

  # Ensure INSTALL_DIR exists
  if [ ! -d "$INSTALL_DIR" ]; then
    if [ -w "$(dirname "$INSTALL_DIR")" ]; then
      mkdir -p "$INSTALL_DIR"
    elif command -v sudo >/dev/null 2>&1; then
      sudo mkdir -p "$INSTALL_DIR"
    else
      echo "Error: INSTALL_DIR ('${INSTALL_DIR}') does not exist and cannot be created." >&2
      echo "Please re-run the installer with a writable INSTALL_DIR, for example:" >&2
      echo "  INSTALL_DIR=\$HOME/.local/bin sh install.sh" >&2
      exit 1
    fi
  fi

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmpdir}/stale" "${INSTALL_DIR}/stale"
    chmod +x "${INSTALL_DIR}/stale"
  else
    if command -v sudo >/dev/null 2>&1; then
      sudo mv "${tmpdir}/stale" "${INSTALL_DIR}/stale"
      sudo chmod +x "${INSTALL_DIR}/stale"
    else
      echo "Error: INSTALL_DIR ('${INSTALL_DIR}') is not writable and 'sudo' is not available." >&2
      echo "Please re-run the installer with a writable INSTALL_DIR, for example:" >&2
      echo "  INSTALL_DIR=\$HOME/.local/bin sh install.sh" >&2
      exit 1
    fi
  fi

  echo "stale ${version} installed to ${INSTALL_DIR}/stale"
}

main
