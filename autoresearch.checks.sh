#!/usr/bin/env bash
set -euo pipefail

echo "=== Checking all example apps compile, run, and produce correct output ==="
count=0
fail=0
for dir in examples/apps/*/; do
  if [ -f "$dir/tonic.toml" ]; then
    name=$(basename "$dir")
    actual=$(TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g') || true

    if [ -z "$actual" ] && ! TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" >/dev/null 2>&1; then
      echo "  FAIL: $name (crashed)"
      fail=$((fail + 1))
      continue
    fi

    if [ -f "$dir/expected_output.txt" ]; then
      expected=$(cat "$dir/expected_output.txt")
      if [ "$actual" = "$expected" ]; then
        echo "  PASS: $name (exact match)"
        count=$((count + 1))
      else
        echo "  FAIL: $name (output mismatch)"
        diff <(echo "$actual") <(echo "$expected") || true
        fail=$((fail + 1))
      fi
    elif [ -f "$dir/expected_patterns.txt" ]; then
      pattern_ok=true
      while IFS= read -r pattern; do
        [ -z "$pattern" ] && continue
        if ! echo "$actual" | grep -qF "$pattern"; then
          echo "  FAIL: $name (missing pattern: $pattern)"
          pattern_ok=false
          break
        fi
      done < "$dir/expected_patterns.txt"
      if [ "$pattern_ok" = true ]; then
        echo "  PASS: $name (patterns matched)"
        count=$((count + 1))
      else
        fail=$((fail + 1))
      fi
    else
      # No expected output file — just check it runs (exit 0)
      if TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" >/dev/null 2>&1; then
        echo "  PASS: $name (runs, no output check)"
        count=$((count + 1))
      else
        echo "  FAIL: $name (crashed)"
        fail=$((fail + 1))
      fi
    fi
  fi
done
echo "correct=$count failed=$fail"

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
  echo "CHECKS FAILED: $fail example(s) do not produce correct output"
  exit 1
fi
echo "ALL CHECKS PASSED"
