#!/usr/bin/env bash
set -euo pipefail

echo "=== Checking all example apps compile and run ==="
count=0
fail=0
for dir in examples/apps/*/; do
  if [ -f "$dir/tonic.toml" ]; then
    name=$(basename "$dir")
    if TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" >/dev/null 2>&1; then
      echo "  PASS: $name"
      count=$((count + 1))
    else
      echo "  FAIL: $name"
      fail=$((fail + 1))
    fi
  fi
done
echo "runnable=$count failed=$fail"

echo ""
echo "=== fmt check ==="
cargo fmt --all -- --check

echo ""
echo "=== clippy check ==="
cargo clippy --all-targets --all-features -- -D warnings

echo ""
echo "=== test check ==="
cargo test

if [ "$fail" -gt 0 ]; then
  echo "CHECKS FAILED: $fail example(s) do not run"
  exit 1
fi
echo "ALL CHECKS PASSED"
