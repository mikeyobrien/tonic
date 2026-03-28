# Context

## Objective
Create backpressure mechanisms that stop parity drift between `tonic run` and `tonic compile` from landing unnoticed.

## Current state
Experiment 1 is complete and **kept**.

### What changed
- `tests/differential_backends.rs` now derives enforced differential coverage from the parity catalog instead of a hand-maintained subset.
- Every active compileable parity fixture now has an explicit differential decision.
- Native C equality emission was corrected to use `tn_runtime_value_equal(...)`, fixing the newly exposed `strict_equality.tn` mismatch.

### Primary metric
- Eligible parity fixtures missing enforced differential coverage
- Baseline: **80** uncovered active compileable fixtures
- Current: **0** uncovered fixtures

### Current coverage snapshot
- Eligible active compileable catalog fixtures: **99**
- Enforced differential fixtures: **91**
- Explicit exclusions: **8**
- Silently uncovered eligible fixtures: **0**

### Evidence collected
1. `cargo test --test differential_backends active_compileable_catalog_entries_have_explicit_differential_coverage -- --nocapture` ✅
2. `cargo test --test differential_backends -- --nocapture` ✅ (`4 passed` in `26.85s`)

## Remaining explicit exclusions
These are known gaps, not silent drift:
- Missing native `tn_runtime_for` support:
  - `examples/parity/02-operators/stepped_range.tn`
  - `examples/parity/10-idiomatic/closures_and_captures.tn`
  - `examples/parity/10-idiomatic/fizzbuzz.tn`
  - `examples/parity/10-idiomatic/keyword_filtering.tn`
  - `examples/parity/10-idiomatic/list_processing.tn`
  - `examples/parity/10-idiomatic/pipeline_transform.tn`
- Multi-clause anonymous-function capture gap:
  - `examples/parity/05-functions/function_capture_multi_clause_anon.tn`
- Native runtime diagnostic text drift:
  - `examples/parity/06-control-flow/for_into_runtime_fail.tn`

## Decision boundary for the next strategist
Default route: **`task.complete`**.

If the loop is already sitting on a `task.complete` routing event and nothing about scope or evidence has changed, stop there rather than re-planning or re-verifying the same slice.

If advisory topology metadata still suggests strategist/implementer/benchmarker/evaluator after `task.complete`, ignore it unless the scope has actually changed.

Only plan another experiment if there is an explicit decision to burn down one of the 8 named exclusions. If that happens, pick exactly one exclusion family and define a fresh baseline metric for it.

## Relevant files
- `.miniloop/autoresearch.md`
- `.miniloop/progress.md`
- `tests/differential_backends.rs`
- `examples/parity/catalog.toml`
- `src/c_backend/ops.rs`
- `src/c_backend/stubs_closures.rs`
- `src/c_backend/terminator.rs`

## Operator note
Previous loop docs here had stale `tonic install` planning context. They should not drive the next iteration.