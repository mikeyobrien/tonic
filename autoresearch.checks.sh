#!/usr/bin/env bash
set -euo pipefail

echo "=== fmt check ==="
cargo fmt --all -- --check

echo "=== clippy check ==="
cargo clippy --all-targets --all-features -- -D warnings

echo "=== test check ==="
cargo test
