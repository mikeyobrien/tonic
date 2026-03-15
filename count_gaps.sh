#!/usr/bin/env bash
set -euo pipefail

oversized=$(find src -name '*.rs' -exec wc -l {} + | awk '$1 > 500 && !/total/ {count++} END {print count+0}')

clippy_err=0
if ! cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -q '^error'; then
  clippy_err=0
else
  clippy_err=1
fi

fmt_diff=0
if ! cargo fmt --all -- --check >/dev/null 2>&1; then
  fmt_diff=1
fi

test_output=$(cargo test 2>&1)
test_fail=$(echo "$test_output" | grep '^test result:' | awk '{sum += $6} END {print sum+0}')

parity_unchecked=$(awk '/^---$/{past_legend=1} past_legend && /^\- \[ \]/' PARITY.md | wc -l | tr -d ' ')
parity_partial=$(awk '/^---$/{past_legend=1} past_legend && /^\- \[~\]/' PARITY.md | wc -l | tr -d ' ')

stdlib_p1=$(grep -c '| P1 |' docs/core-stdlib-gap-list.md 2>/dev/null || true)
stdlib_p1=${stdlib_p1:-0}

todos=$({ grep -r 'TODO\|FIXME\|HACK\|XXX' src/ --include='*.rs' || true; } | wc -l | tr -d ' ')

total=$((oversized + clippy_err + fmt_diff + test_fail + parity_unchecked + parity_partial + stdlib_p1 + todos))

echo "oversized=$oversized clippy=$clippy_err fmt=$fmt_diff test_fail=$test_fail parity_unchecked=$parity_unchecked parity_partial=$parity_partial stdlib_p1=$stdlib_p1 todos=$todos"
echo "TOTAL_GAPS=$total"
