#!/usr/bin/env bash
set -euo pipefail

manifest="${1:-benchmarks/native-compiler-suite.toml}"
compile_latency_ms="${TONIC_COMPILE_LATENCY_MS:-2600}"

printf 'Building release binaries...\n'
cargo build --release -q

printf 'Running native compiler contract in enforce mode...\n'
target/release/benchsuite \
  --bin target/release/tonic \
  --manifest "$manifest" \
  --target-name interpreter \
  --compile-latency-ms "$compile_latency_ms" \
  --enforce
