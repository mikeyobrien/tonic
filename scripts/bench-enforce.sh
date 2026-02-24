#!/usr/bin/env bash
set -euo pipefail

echo "Building release binary..."
cargo build --release -q

echo "Running benchmarks in enforce mode..."
target/release/benchsuite \
  --bin target/release/tonic \
  --manifest benchmarks/suite.toml \
  --runs 15 \
  --warmup 3 \
  --enforce
