#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${TONIC_PARITY_ARTIFACT_DIR:-.tonic/parity}"
catalog="${TONIC_PARITY_CATALOG:-examples/parity/catalog.toml}"
report_json="${TONIC_PARITY_REPORT_JSON:-$artifact_dir/llvm-catalog-parity.json}"
report_md="${TONIC_PARITY_REPORT_MD:-$artifact_dir/llvm-catalog-parity.md}"
parity_bin="${TONIC_PARITY_BIN:-target/debug/llvm_catalog_parity}"
tonic_bin="${TONIC_PARITY_TONIC_BIN:-target/debug/tonic}"

mkdir -p "$artifact_dir"

if [[ ! -x "$parity_bin" || ! -x "$tonic_bin" ]]; then
  printf 'Building debug tonic + llvm_catalog_parity binaries...\n'
  cargo build -q --bin tonic --bin llvm_catalog_parity
fi

printf 'Running LLVM catalog parity gate in enforce mode...\n'
cmd=(
  "$parity_bin"
  --catalog "$catalog"
  --report-json "$report_json"
  --report-md "$report_md"
  --tonic-bin "$tonic_bin"
  --enforce
)

"${cmd[@]}"

printf 'LLVM parity reports:\n  %s\n  %s\n' "$report_json" "$report_md"
