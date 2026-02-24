#!/usr/bin/env bash
set -euo pipefail

manifest="${1:-benchmarks/interpreter-vs-compiled-suite.toml}"
runs="${TONIC_BENCH_RUNS:-20}"
warmup="${TONIC_BENCH_WARMUP:-5}"
json_out="${TONIC_BENCH_JSON_OUT:-benchmarks/interpreter-vs-compiled-summary.json}"
markdown_out="${TONIC_BENCH_MARKDOWN_OUT:-benchmarks/interpreter-vs-compiled-summary.md}"
enforce="${TONIC_BENCH_ENFORCE:-0}"

mkdir -p "$(dirname "$json_out")"
mkdir -p "$(dirname "$markdown_out")"

printf 'Building release binaries...\n'
cargo build --release -q

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

"${cmd[@]}"
