# Task: Pattern Runtime Completion (List/Map)

## Goal
Complete list/map pattern lowering and runtime matching semantics.

## Scope
- Lower map/list patterns to IR (remove current map-pattern lowering failure path).
- Runtime matcher support for list and map values/patterns.
- Pattern binding behavior for nested list/map patterns.
- Deterministic no-match diagnostic path.

## Out of Scope
- Pin operator and guards (next task).

## Deliverables
- IR pattern shape expansion.
- Runtime `case` execution that handles list/map patterns.

## Acceptance Criteria
- `case` with list/map patterns executes correctly.
- Non-matching patterns fail with deterministic diagnostics.

## Verification
- `cargo test`
- Add tests to `tests/check_dump_ir_result_case.rs` + new `run_case_*` smoke tests.

## Suggested Commit
`feat(parity): implement list and map pattern matching at runtime`
