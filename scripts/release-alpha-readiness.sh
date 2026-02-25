#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' "Usage: scripts/release-alpha-readiness.sh [--version <semver>]"
}

info() {
  printf 'alpha-readiness: %s\n' "$1"
}

fail() {
  printf 'alpha-readiness: fail: %s\n' "$1" >&2
  exit 1
}

extract_manifest_version() {
  local manifest_path="$1"
  local version

  version="$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$manifest_path" || true)"
  if [[ -z "$version" ]]; then
    fail "unable to parse package version from $manifest_path"
  fi

  printf '%s\n' "$version"
}

if [[ $# -gt 2 ]]; then
  usage >&2
  exit 64
fi

target_version=""
if [[ $# -eq 1 ]]; then
  if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    usage
    exit 0
  fi

  usage >&2
  exit 64
fi

if [[ $# -eq 2 ]]; then
  if [[ "$1" != "--version" || -z "$2" ]]; then
    usage >&2
    exit 64
  fi
  target_version="$2"
fi

manifest_path="${TONIC_ALPHA_MANIFEST:-Cargo.toml}"
changelog_path="${TONIC_ALPHA_CHANGELOG:-CHANGELOG.md}"
artifact_dir="${TONIC_ALPHA_ARTIFACT_DIR:-.tonic/native-gates}"
native_gates_cmd="${TONIC_NATIVE_GATES_CMD:-./scripts/native-gates.sh}"

if [[ ! -f "$manifest_path" ]]; then
  fail "manifest not found at $manifest_path"
fi

if [[ -z "$target_version" ]]; then
  target_version="$(extract_manifest_version "$manifest_path")"
fi

if [[ "$target_version" != *-alpha* ]]; then
  fail "target version must include '-alpha' prerelease label: $target_version"
fi

info "target version: $target_version"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  fail "current directory is not a git repository"
fi

if [[ -n "$(git status --porcelain)" ]]; then
  fail "git working tree must be clean before release validation"
fi

info "git working tree is clean"

if [[ ! -f "$changelog_path" ]]; then
  fail "CHANGELOG.md not found at $changelog_path"
fi

if ! grep -Fq "## [$target_version]" "$changelog_path"; then
  fail "CHANGELOG.md is missing version heading: ## [$target_version]"
fi

info "changelog contains version heading"

info "running native gates command: $native_gates_cmd"
if ! TONIC_NATIVE_ARTIFACT_DIR="$artifact_dir" "$native_gates_cmd"; then
  fail "native gates command failed: $native_gates_cmd"
fi

required_artifacts=(
  "native-compiler-summary.json"
  "native-compiler-summary.md"
  "native-compiled-summary.json"
  "native-compiled-summary.md"
)

for artifact in "${required_artifacts[@]}"; do
  artifact_path="$artifact_dir/$artifact"
  if [[ ! -s "$artifact_path" ]]; then
    fail "required artifact missing or empty: $artifact_path"
  fi
  info "artifact ok: $artifact_path"
done

info "pass: alpha release readiness checks succeeded"
