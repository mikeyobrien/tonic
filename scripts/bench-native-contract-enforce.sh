#!/usr/bin/env bash
set -euo pipefail

manifest="${1:-benchmarks/native-compiler-suite.toml}"
compile_latency_ms="${TONIC_COMPILE_LATENCY_MS:-2600}"
runs="${TONIC_BENCH_RUNS:-15}"
warmup="${TONIC_BENCH_WARMUP:-3}"
target_name="${TONIC_BENCH_TARGET_NAME:-interpreter}"
json_out="${TONIC_BENCH_JSON_OUT:-benchmarks/native-compiler-summary.json}"
markdown_out="${TONIC_BENCH_MARKDOWN_OUT:-benchmarks/native-compiler-summary.md}"
enforce="${TONIC_BENCH_ENFORCE:-1}"

mkdir -p "$(dirname "$json_out")"
mkdir -p "$(dirname "$markdown_out")"

printf 'Building release binaries...\n'
cargo build --release -q

printf 'Running native compiler contract benchmark...\n'
cmd=(
  target/release/benchsuite
  --bin target/release/tonic
  --manifest "$manifest"
  --runs "$runs"
  --warmup "$warmup"
  --target-name "$target_name"
  --compile-latency-ms "$compile_latency_ms"
  --json-out "$json_out"
  --markdown-out "$markdown_out"
)

if [[ "$enforce" == "1" ]]; then
  cmd+=(--enforce)
fi

"${cmd[@]}"
