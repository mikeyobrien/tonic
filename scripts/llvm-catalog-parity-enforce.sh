#!/usr/bin/env bash
# LLVM backend is experimental. Parity failures are informational only and do
# not block CI. Set TONIC_LLVM_PARITY_ENFORCE=1 to restore blocking behaviour.
set -euo pipefail

artifact_dir="${TONIC_PARITY_ARTIFACT_DIR:-.tonic/parity}"
catalog="${TONIC_PARITY_CATALOG:-examples/parity/catalog.toml}"
report_json="${TONIC_PARITY_REPORT_JSON:-$artifact_dir/llvm-catalog-parity.json}"
report_md="${TONIC_PARITY_REPORT_MD:-$artifact_dir/llvm-catalog-parity.md}"
parity_bin="${TONIC_PARITY_BIN:-target/debug/llvm_catalog_parity}"
tonic_bin="${TONIC_PARITY_TONIC_BIN:-target/debug/tonic}"
enforce="${TONIC_LLVM_PARITY_ENFORCE:-0}"

mkdir -p "$artifact_dir"

if [[ ! -x "$parity_bin" || ! -x "$tonic_bin" ]]; then
  printf 'Building debug tonic + llvm_catalog_parity binaries...\n'
  cargo build -q --bin tonic --bin llvm_catalog_parity
fi

if [[ "$enforce" == "1" ]]; then
  printf 'Running LLVM catalog parity gate in enforce mode (TONIC_LLVM_PARITY_ENFORCE=1)...\n'
  cmd=(
    "$parity_bin"
    --catalog "$catalog"
    --report-json "$report_json"
    --report-md "$report_md"
    --tonic-bin "$tonic_bin"
    --enforce
  )
else
  printf 'Running LLVM catalog parity gate in informational mode (experimental backend)...\n'
  cmd=(
    "$parity_bin"
    --catalog "$catalog"
    --report-json "$report_json"
    --report-md "$report_md"
    --tonic-bin "$tonic_bin"
  )
fi

# Run parity check; capture exit code without aborting the script so that
# informational mode never blocks the pipeline.
set +e
"${cmd[@]}"
parity_exit=$?
set -e

if [[ $parity_exit -ne 0 ]]; then
  if [[ "$enforce" == "1" ]]; then
    printf 'LLVM parity enforce failed (exit %d).\n' "$parity_exit"
    exit "$parity_exit"
  else
    printf 'LLVM parity mismatches detected (informational only — backend is experimental).\n'
  fi
fi

printf 'LLVM parity reports:\n  %s\n  %s\n' "$report_json" "$report_md"
