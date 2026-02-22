# Task: Control Forms (`if`/`unless`/`cond`/`with`)

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Implement missing control-flow forms needed for idiomatic Elixir-like programs.

## Scope
- `if` and `unless`.
- `cond` with ordered branch evaluation.
- `with` chaining for result-style workflows (minimum practical subset).
- Lowering/runtime semantics and diagnostics.

## Out of Scope
- Full exception stacktrace parity or macro-expansion fidelity.

## Deliverables
- Parser + IR + runtime support for all forms above.
- Integration tests demonstrating equivalent behavior to existing `case`-based logic.

## Acceptance Criteria
- `if/unless/cond` evaluate expected branches.
- `with` supports happy-path chaining and fallback handling for supported shapes.

## Verification
- `cargo test`
- Add new integration tests under `tests/run_*` and `tests/check_dump_ast_*`.

## Suggested Commit
`feat(parity): add if unless cond and with control forms`
