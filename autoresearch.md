# Autoresearch: Production Readiness of the Tonic Language

## Goal

Improve Tonic's production readiness through an autonomous experiment loop.
Each iteration should close a concrete gap: code quality, test coverage,
stdlib completeness, parity, documentation accuracy, or structural health.

## Primary Metric

`readiness_gaps` — count of measurable production-readiness deficits.
Direction: lower is better. Zero means all tracked gaps are closed.

### Gap categories tracked

1. **Oversized files** — source files in `src/` exceeding 500 lines (project policy)
2. **Clippy/fmt violations** — any warnings or format drift
3. **Test failures** — any failing tests in `cargo test`
4. **Differential correctness failures** — interpreter vs compiled backend mismatches
5. **PARITY.md unchecked items** — `[ ]` items in the syntax parity checklist
6. **PARITY.md partial items** — `[~]` items that need completion
7. **Stdlib gap P1 items** — P1 gaps from `docs/core-stdlib-gap-list.md`
8. **Single TODO/FIXME/HACK/XXX markers** — unresolved code debt

### How gaps are counted

```bash
# Oversized files (>500 lines in src/)
find src -name '*.rs' | xargs wc -l | awk '$1 > 500 && !/total/ {count++} END {print count}'

# Clippy (0 or 1)
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep -c '^error'

# Fmt (0 or 1)
cargo fmt --all -- --check 2>&1 | grep -c '^Diff'

# Test failures
cargo test 2>&1 | grep '^test result:' | awk '{sum += $6} END {print sum}'

# Parity unchecked (exclude legend section)
awk '/^---$/{past_legend=1} past_legend && /^\- \[ \]/' PARITY.md | wc -l

# Parity partial (exclude legend section)
awk '/^---$/{past_legend=1} past_legend && /^\- \[~\]/' PARITY.md | wc -l

# Stdlib P1 gaps
grep -c '| P1 |' docs/core-stdlib-gap-list.md 2>/dev/null || echo 0

# TODO/FIXME/HACK/XXX
grep -r 'TODO\|FIXME\|HACK\|XXX' src/ --include='*.rs' | wc -l
```

## Checks

The experiment passes if:
1. `cargo fmt --all -- --check` exits 0
2. `cargo clippy --all-targets --all-features -- -D warnings` exits 0
3. `cargo test` exits 0

## Current Best

**readiness_gaps = 4** (Run 17) — down from baseline of 20

## What's Been Tried

- **Run 1 (DISCARD, metric=20)**: Baseline measurement — 18 oversized files, 2 partial parity items.
- **Run 2 (KEEP, metric=19)**: Split typing.rs (782→294 lines). Hypothesis: splitting oversized files reduces gap count — confirmed.
- **Run 3 (KEEP, metric=18)**: Split resolver.rs (1086→401 lines). Hypothesis: continued file splitting — confirmed.
- **Run 4 (KEEP, metric=17)**: Split llvm_backend/tests.rs (739→395 lines). Hypothesis: continued — confirmed.
- **Run 5 (KEEP, metric=16)**: Split parser/tests.rs (1213→490 lines). Hypothesis: continued — confirmed.
- **Run 6 (KEEP, metric=15)**: Split manifest.rs (1239→414 lines). Hypothesis: continued — confirmed.
- **Run 7 (KEEP, metric=14)**: Split stubs_for.rs (778→406 lines). Hypothesis: continued — confirmed.
- **Run 8 (KEEP, metric=13)**: Split interop.rs (1580→199 lines). Hypothesis: continued — confirmed.
- **Run 9 (KEEP, metric=12)**: Split stubs_try.rs (562→293 lines). Hypothesis: continued — confirmed.
- **Run 10 (KEEP, metric=11)**: Split runtime_patterns.rs (574→345 lines). Hypothesis: continued — confirmed.
- **Run 11 (KEEP, metric=10)**: Split interop/http_server.rs (1407→484 lines). Hypothesis: continued — confirmed.
- **Run 12 (KEEP, metric=9)**: Split interop/system.rs (1445→481 lines). Hypothesis: continued — confirmed.
- **Run 13 (KEEP, metric=8)**: Split parser/ast.rs (1112→473 lines). Hypothesis: continued — confirmed.
- **Run 14 (KEEP, metric=7)**: Split runtime.rs (1673→289 lines). Hypothesis: continued — confirmed.
- **Run 15 (KEEP, metric=6)**: Split llvm_backend/codegen.rs (1716→349 lines). Hypothesis: continued — confirmed.
- **Run 16 (KEEP, metric=5)**: Split ir.rs (2243→413 lines). Hypothesis: continued — confirmed.
- **Run 17 (KEEP, metric=4)**: Split main.rs into cmd_*.rs handlers + split lexer.rs (2083 lines) into directory module with 5 files. Hypothesis: continued — confirmed.

## Rules

- Do not overfit to benchmarks or cheat on benchmarks.
- Each iteration should close exactly one gap or a small coherent group.
- Commit when tests pass.
- Keep implementation files at 500 lines or less.
- Treat dead code and clippy warnings as hard blockers.
- Do not break existing tests to reduce gap count.
- Prefer structural improvements over cosmetic changes.
- When splitting oversized files, preserve all existing functionality and tests.
