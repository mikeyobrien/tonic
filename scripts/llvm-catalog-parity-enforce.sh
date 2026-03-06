#!/usr/bin/env bash
# LLVM backend is experimental. Parity failures are informational only and do
# not block CI. Set TONIC_LLVM_PARITY_ENFORCE=1 to restore blocking behaviour.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/observability.sh
source "$script_dir/lib/observability.sh"
tonic_obs_script_init "llvm-catalog-parity-enforce" "$@"
trap 'tonic_obs_finish "$?"' EXIT

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
  tonic_obs_run_step 'cargo build -q --bin tonic --bin llvm_catalog_parity' \
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
  step_name='llvm_catalog_parity --enforce'
else
  printf 'Running LLVM catalog parity gate in informational mode (experimental backend)...\n'
  cmd=(
    "$parity_bin"
    --catalog "$catalog"
    --report-json "$report_json"
    --report-md "$report_md"
    --tonic-bin "$tonic_bin"
  )
  step_name='llvm_catalog_parity'
fi

# Run parity check; capture exit code without aborting the script so that
# informational mode never blocks the pipeline.
set +e
tonic_obs_run_step "$step_name" "${cmd[@]}"
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

tonic_obs_record_artifact 'llvm-parity-report-json' "$report_json"
tonic_obs_record_artifact 'llvm-parity-report-md' "$report_md"
printf 'LLVM parity reports:\n  %s\n  %s\n' "$report_json" "$report_md"
