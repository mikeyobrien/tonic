# Autoresearch Progress

## Status
Experiment 1 evaluated and kept. Objective satisfied; routing to `task.complete`.

## Stop condition
If the latest routing event is already `task.complete` and no new scope has been introduced, do not reopen planning or re-run validation just to restate the result. Prefer a brief memory note if needed, then exit cleanly.

The topology block is advisory only. If it still lists generic roles or allowed events after `task.complete`, treat that as stale/default routing metadata, not as a reason to do more work.

## Current experiment
- **Hypothesis:** parity drift is slipping through because `tests/differential_backends.rs` used a hand-maintained subset instead of catalog-driven coverage rules.
- **Primary metric:** eligible parity fixtures missing enforced differential coverage (lower is better).
- **Baseline:** 80 uncovered fixtures.
- **Current implementation result:** 0 uncovered fixtures via 91 enforced differential fixtures plus 8 explicit catalog exclusions.

## Implemented changes
- Replaced the hardcoded `PARITY_DIFF_SUBSET` gate with catalog-driven selection of every active catalog entry where `check_exit = 0`.
- Added an explicit exclusion list with required reasons so unsupported or known-drifting fixtures must be acknowledged instead of silently falling out of coverage.
- Fixed native C backend equality emission to use `tn_runtime_value_equal(...)` for equality/inequality comparisons in normal ops, closure lowering, and guard/dispatcher code paths.

## Explicit exclusions introduced
- `examples/parity/02-operators/stepped_range.tn` — native C backend still aborts on `tn_runtime_for`-backed comprehensions.
- `examples/parity/05-functions/function_capture_multi_clause_anon.tn` — native closure lowering still rejects this multi-clause anonymous function capture.
- `examples/parity/06-control-flow/for_into_runtime_fail.tn` — native runtime failure output still lacks interpreter-style source-context diagnostics.
- `examples/parity/10-idiomatic/closures_and_captures.tn`
- `examples/parity/10-idiomatic/fizzbuzz.tn`
- `examples/parity/10-idiomatic/keyword_filtering.tn`
- `examples/parity/10-idiomatic/list_processing.tn`
- `examples/parity/10-idiomatic/pipeline_transform.tn`
  - The last five are also excluded because they still hit the missing `tn_runtime_for` native helper.

## Changed files
- `tests/differential_backends.rs`
- `src/c_backend/ops.rs`
- `src/c_backend/stubs_closures.rs`
- `src/c_backend/terminator.rs`

## Correctness evidence collected
1. `cargo test --test differential_backends active_compileable_catalog_entries_have_explicit_differential_coverage -- --nocapture` ✅
   - Result: 1 passed; every active compileable catalog entry still has an explicit differential decision.
2. `cargo test --test differential_backends -- --nocapture` ✅
   - Result: 4 passed in 26.85s; the full differential backend suite still holds after the catalog-driven gate expansion.

## Raw measurement
- Baseline uncovered eligible fixtures: 80
- Current eligible fixtures: 99
- Current enforced differential fixtures: 91
- Current explicit exclusions: 8
- Current uncovered eligible fixtures: 0
- Net change in uncovered eligible fixtures: 80 -> 0
- Remaining backpressure gap: only the 8 explicit exclusions, each documented with an inline reason in `tests/differential_backends.rs`

## Verdict
keep

## Evaluation
- The primary metric improved decisively: uncovered eligible fixtures dropped from 80 to 0.
- Correctness evidence is strong enough for a keep: the new explicit-coverage contract test passed, and the full differential backend suite passed after expanding coverage.
- The 8 remaining gaps are no longer silent drift; they are enforced, named exclusions with concrete reasons. That satisfies the backpressure objective of blocking unnoticed parity regressions from landing.
- The equality-emission fix is not speculative cleanup; it resolved a real newly-exposed `strict_equality.tn` native mismatch while broadening the enforced gate.

## Next role
completion

## Next action
Emit `task.complete`: catalog-driven differential coverage now enforces an explicit include/exclude decision for every active compileable parity fixture, and the remaining 8 gaps are acknowledged exclusions instead of silent drift.
