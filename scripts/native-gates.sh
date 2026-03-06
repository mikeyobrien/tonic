#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/observability.sh
source "$script_dir/lib/observability.sh"
tonic_obs_script_init "native-gates" "$@"
trap 'tonic_obs_finish "$?"' EXIT

artifact_dir="${TONIC_NATIVE_ARTIFACT_DIR:-.tonic/native-gates}"
manifest="${TONIC_NATIVE_BENCH_MANIFEST:-benchmarks/native-compiler-suite.toml}"
summary_json="${TONIC_NATIVE_SUMMARY_JSON:-$artifact_dir/native-compiler-summary.json}"
summary_md="${TONIC_NATIVE_SUMMARY_MD:-$artifact_dir/native-compiler-summary.md}"
compiled_manifest="${TONIC_NATIVE_COMPILED_BENCH_MANIFEST:-benchmarks/native-compiled-suite.toml}"
compiled_summary_json="${TONIC_NATIVE_COMPILED_SUMMARY_JSON:-$artifact_dir/native-compiled-summary.json}"
compiled_summary_md="${TONIC_NATIVE_COMPILED_SUMMARY_MD:-$artifact_dir/native-compiled-summary.md}"

mkdir -p "$artifact_dir"

printf '%s\n' '[native-gates] cargo fmt --all -- --check'
tonic_obs_run_step 'cargo fmt --all -- --check' cargo fmt --all -- --check

printf '%s\n' '[native-gates] cargo clippy --all-targets --all-features -- -D warnings'
tonic_obs_run_step 'cargo clippy --all-targets --all-features -- -D warnings' \
  cargo clippy --all-targets --all-features -- -D warnings

printf '%s\n' '[native-gates] cargo test'
tonic_obs_run_step 'cargo test' cargo test

printf '%s\n' '[native-gates] scripts/differential-enforce.sh'
tonic_obs_run_step 'scripts/differential-enforce.sh' "$script_dir/differential-enforce.sh"

printf '%s\n' '[native-gates] scripts/llvm-catalog-parity-enforce.sh (experimental - informational only)'
tonic_obs_run_step 'scripts/llvm-catalog-parity-enforce.sh' "$script_dir/llvm-catalog-parity-enforce.sh"

printf '%s\n' '[native-gates] scripts/bench-native-contract-enforce.sh (interpreter)'
tonic_obs_run_step 'scripts/bench-native-contract-enforce.sh (interpreter)' \
  env \
    TONIC_BENCH_JSON_OUT="$summary_json" \
    TONIC_BENCH_MARKDOWN_OUT="$summary_md" \
    TONIC_BENCH_ENFORCE=0 \
    "$script_dir/bench-native-contract-enforce.sh" "$manifest"
tonic_obs_record_artifact 'benchmark-summary-json' "$summary_json"
tonic_obs_record_artifact 'benchmark-summary-md' "$summary_md"

printf '%s\n' '[native-gates] scripts/native-regression-policy.sh --mode strict (interpreter)'
tonic_obs_run_step 'scripts/native-regression-policy.sh --mode strict (interpreter)' \
  "$script_dir/native-regression-policy.sh" "$summary_json" --mode strict

printf '%s\n' '[native-gates] scripts/bench-native-contract-enforce.sh (compiled)'
tonic_obs_run_step 'scripts/bench-native-contract-enforce.sh (compiled)' \
  env \
    TONIC_BENCH_JSON_OUT="$compiled_summary_json" \
    TONIC_BENCH_MARKDOWN_OUT="$compiled_summary_md" \
    TONIC_BENCH_TARGET_NAME='compiled' \
    TONIC_BENCH_ENFORCE=0 \
    "$script_dir/bench-native-contract-enforce.sh" "$compiled_manifest"
tonic_obs_record_artifact 'compiled-benchmark-summary-json' "$compiled_summary_json"
tonic_obs_record_artifact 'compiled-benchmark-summary-md' "$compiled_summary_md"

printf '%s\n' '[native-gates] scripts/native-regression-policy.sh --mode strict (compiled)'
tonic_obs_run_step 'scripts/native-regression-policy.sh --mode strict (compiled)' \
  "$script_dir/native-regression-policy.sh" "$compiled_summary_json" --mode strict

memory_bakeoff_artifact_dir="${TONIC_MEMORY_BAKEOFF_ARTIFACT_DIR:-$artifact_dir/memory-bakeoff}"
printf '%s\n' '[native-gates] scripts/memory-bakeoff.sh --ci'
tonic_obs_run_step 'scripts/memory-bakeoff.sh --ci' \
  env TONIC_MEMORY_BAKEOFF_ARTIFACT_DIR="$memory_bakeoff_artifact_dir" \
  "$script_dir/memory-bakeoff.sh" --ci
tonic_obs_record_artifact 'memory-bakeoff-dir' "$memory_bakeoff_artifact_dir"
if [[ -f "$memory_bakeoff_artifact_dir/summary.tsv" ]]; then
  tonic_obs_record_artifact 'memory-bakeoff-summary-tsv' "$memory_bakeoff_artifact_dir/summary.tsv"
fi
if [[ -f "$memory_bakeoff_artifact_dir/summary.md" ]]; then
  tonic_obs_record_artifact 'memory-bakeoff-summary-md' "$memory_bakeoff_artifact_dir/summary.md"
fi

printf '[native-gates] complete. benchmark artifacts:\n  %s\n  %s\n  %s\n  %s\n  %s\n' \
  "$summary_json" \
  "$summary_md" \
  "$compiled_summary_json" \
  "$compiled_summary_md" \
  "$memory_bakeoff_artifact_dir"
