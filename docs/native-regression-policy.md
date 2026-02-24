# Native Regression Policy (Rust/Go Comparative Gates)

This policy defines how Tonic evaluates native benchmark regressions from `benchmarks/native-compiler-suite.toml`.

## Inputs

- Benchmark contract report: `native-compiler-summary.json`
- Evaluator: `scripts/native-regression-policy.sh <summary.json> [--mode strict|advisory]`

## Verdicts

The policy script emits exactly one verdict and exits with a deterministic status code:

- `verdict=pass` (exit `0`)
  - Performance contract is green.
- `verdict=quarantine` (exit `2` in strict mode, `0` in advisory mode)
  - Small regression that can be investigated without immediate rollback.
- `verdict=rollback` (exit `3`)
  - Significant regression or reliability risk; candidate must be rejected.

## Allowed Variance / Quarantine Window

Given `relative_budget_pct` from the benchmark manifest:

- `budget_ratio = 1 + relative_budget_pct/100`
- `quarantine_ratio = budget_ratio + 0.10`
- `rollback_ratio = budget_ratio + 0.20`

A candidate is **quarantine** when all are true:

1. No SLO failure (`performance_contract.slo.status != fail`)
2. No hard regression (`ratio > rollback_ratio`)
3. At most two workloads exceed `budget_ratio`
4. `overall_score` is within `0.03` of `pass_threshold`

Otherwise, non-pass outcomes are **rollback**.

## Rollback Criteria

A candidate is forced to rollback when any is true:

- SLO status is fail or SLO failures are present
- Any hard regression (`ratio > rollback_ratio`)
- `overall_score` misses threshold by more than `0.08`
- Contract report is missing/invalid

## CI Policy

- PR/main CI uses **strict mode**.
- Quarantine and rollback both fail CI and block merge.
- Benchmark JSON/Markdown artifacts are always uploaded for inspection.

## Release Candidate Policy

- Candidate runs also use **strict mode**.
- Tag/release cut requires `verdict=pass`.
- If quarantine/rollback occurs, open a remediation PR and re-run release gates.

## Local Reproduction

Run the same gate sequence as CI:

```bash
./scripts/native-gates.sh
```

Or benchmark + policy only:

```bash
TONIC_BENCH_ENFORCE=0 \
TONIC_BENCH_JSON_OUT=.tonic/native-gates/native-compiler-summary.json \
TONIC_BENCH_MARKDOWN_OUT=.tonic/native-gates/native-compiler-summary.md \
./scripts/bench-native-contract-enforce.sh

./scripts/native-regression-policy.sh \
  .tonic/native-gates/native-compiler-summary.json --mode strict
```
