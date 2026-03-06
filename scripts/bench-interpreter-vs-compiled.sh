#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/observability.sh
source "$script_dir/lib/observability.sh"
tonic_obs_script_init "bench-interpreter-vs-compiled" "$@"
trap 'tonic_obs_finish "$?"' EXIT

manifest="${1:-benchmarks/interpreter-vs-compiled-suite.toml}"
runs="${TONIC_BENCH_RUNS:-20}"
warmup="${TONIC_BENCH_WARMUP:-5}"
json_out="${TONIC_BENCH_JSON_OUT:-benchmarks/interpreter-vs-compiled-summary.json}"
markdown_out="${TONIC_BENCH_MARKDOWN_OUT:-benchmarks/interpreter-vs-compiled-summary.md}"
enforce="${TONIC_BENCH_ENFORCE:-0}"

mkdir -p "$(dirname "$json_out")"
mkdir -p "$(dirname "$markdown_out")"

printf 'Building release binaries...\n'
tonic_obs_run_step 'cargo build --release -q' cargo build --release -q

printf 'Running interpreter-vs-compiled benchmark suite...\n'
cmd=(
  target/release/benchsuite
  --bin target/release/tonic
  --manifest "$manifest"
  --runs "$runs"
  --warmup "$warmup"
  --json-out "$json_out"
  --markdown-out "$markdown_out"
)

if [[ "$enforce" == "1" ]]; then
  cmd+=(--enforce)
fi

tonic_obs_run_step 'target/release/benchsuite' "${cmd[@]}"
tonic_obs_record_artifact 'benchmark-summary-json' "$json_out"
tonic_obs_record_artifact 'benchmark-summary-md' "$markdown_out"
