#!/usr/bin/env bash
#
# Tonic installer — downloads the appropriate release binary for your platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mikeyobrien/tonic/main/install.sh | bash
#   curl -fsSL ... | bash -s -- --version v0.1.0-alpha.1
#   curl -fsSL ... | bash -s -- --dry-run
#
# Flags:
#   --version VERSION   Install a specific version (default: latest)
#   --install-dir DIR   Override install directory
#   --dry-run           Show what would be done without making changes

set -euo pipefail

REPO="mikeyobrien/tonic"
VERSION=""
INSTALL_DIR=""
DRY_RUN=false

# --- Argument parsing ---

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# --- Platform detection ---

detect_platform() {
  local os arch target

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    MINGW*|MSYS*|CYGWIN*)
      os="pc-windows-msvc"
      ;;
    *)
      echo "Error: unsupported operating system: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)
      echo "Error: unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  target="${arch}-${os}"

  # Windows only supports x86_64
  if [[ "$os" == "pc-windows-msvc" && "$arch" != "x86_64" ]]; then
    echo "Error: Windows builds are only available for x86_64" >&2
    exit 1
  fi

  echo "$target"
}

# --- Version resolution ---

resolve_version() {
  if [[ -n "$VERSION" ]]; then
    # Ensure version starts with 'v'
    if [[ "$VERSION" != v* ]]; then
      VERSION="v${VERSION}"
    fi
    echo "$VERSION"
    return
  fi

  local latest
  latest="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"

  if [[ -z "$latest" ]]; then
    echo "Error: could not determine latest version" >&2
    exit 1
  fi

  echo "$latest"
}

# --- Install directory ---

resolve_install_dir() {
  if [[ -n "$INSTALL_DIR" ]]; then
    echo "$INSTALL_DIR"
    return
  fi

  local dir="$HOME/.local/bin"
  if [[ -d "$dir" && -w "$dir" ]]; then
    echo "$dir"
  else
    echo "/usr/local/bin"
  fi
}

# --- Main ---

main() {
  local target version install_dir archive_name archive_url checksums_url
  local tmpdir use_sudo

  target="$(detect_platform)"
  version="$(resolve_version)"
  install_dir="$(resolve_install_dir)"

  if [[ "$target" == *windows* ]]; then
    archive_name="tonic-${version}-${target}.zip"
  else
    archive_name="tonic-${version}-${target}.tar.gz"
  fi

  local base_url="https://github.com/${REPO}/releases/download/${version}"
  archive_url="${base_url}/${archive_name}"
  checksums_url="${base_url}/checksums.sha256"

  # Determine if sudo is needed
  use_sudo=""
  if [[ -d "$install_dir" && ! -w "$install_dir" ]]; then
    use_sudo="sudo"
  elif [[ ! -d "$install_dir" ]]; then
    # Check if we can create the directory
    local parent="$install_dir"
    while [[ ! -d "$parent" ]]; do
      parent="$(dirname "$parent")"
    done
    if [[ ! -w "$parent" ]]; then
      use_sudo="sudo"
    fi
  fi

  echo "Tonic installer"
  echo "  Version:     ${version}"
  echo "  Target:      ${target}"
  echo "  Archive:     ${archive_name}"
  echo "  Install dir: ${install_dir}"
  if [[ -n "$use_sudo" ]]; then
    echo "  Note:        sudo required for ${install_dir}"
  fi
  echo ""

  if [[ "$DRY_RUN" == true ]]; then
    echo "[dry-run] Would download: ${archive_url}"
    echo "[dry-run] Would verify checksum from: ${checksums_url}"
    echo "[dry-run] Would install tonic to: ${install_dir}/tonic"
    exit 0
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  echo "Downloading ${archive_name}..."
  curl -fsSL -o "${tmpdir}/${archive_name}" "$archive_url"

  echo "Downloading checksums..."
  curl -fsSL -o "${tmpdir}/checksums.sha256" "$checksums_url"

  echo "Verifying checksum..."
  local expected actual
  expected="$(grep "${archive_name}" "${tmpdir}/checksums.sha256" | awk '{print $1}')"
  if [[ -z "$expected" ]]; then
    echo "Error: archive not found in checksums file" >&2
    exit 1
  fi

  if command -v sha256sum &>/dev/null; then
    actual="$(sha256sum "${tmpdir}/${archive_name}" | awk '{print $1}')"
  elif command -v shasum &>/dev/null; then
    actual="$(shasum -a 256 "${tmpdir}/${archive_name}" | awk '{print $1}')"
  else
    echo "Warning: no sha256sum or shasum found, skipping checksum verification" >&2
    actual="$expected"
  fi

  if [[ "$actual" != "$expected" ]]; then
    echo "Error: checksum mismatch" >&2
    echo "  Expected: ${expected}" >&2
    echo "  Actual:   ${actual}" >&2
    exit 1
  fi
  echo "Checksum OK."

  echo "Extracting..."
  if [[ "$archive_name" == *.tar.gz ]]; then
    tar xzf "${tmpdir}/${archive_name}" -C "${tmpdir}"
  else
    unzip -q "${tmpdir}/${archive_name}" -d "${tmpdir}"
  fi

  echo "Installing to ${install_dir}..."
  $use_sudo mkdir -p "$install_dir"
  $use_sudo install -m 755 "${tmpdir}/tonic" "${install_dir}/tonic"

  echo ""
  echo "Tonic ${version} installed to ${install_dir}/tonic"

  # Check if install dir is in PATH
  case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *)
      echo ""
      echo "Note: ${install_dir} is not in your PATH."
      echo "Add it with:  export PATH=\"${install_dir}:\$PATH\""
      ;;
  esac
}

main
