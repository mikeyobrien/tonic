# Task: Operators Logical + Collection

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Add logical and collection-centric operators required for idiomatic Elixir-style code.

## Scope
- Logical: `and`, `or`, `not`, `&&`, `||`, `!`.
- Collection/string: `<>`, `++`, `--`.
- Membership/range: `in`, `..`.
- Parser precedence/associativity alignment.

## Out of Scope
- Macro rewrites or optimizer passes.

## Deliverables
- Parser + IR + runtime + typing support for new operators.
- Compatibility tests for precedence and short-circuit behavior.

## Acceptance Criteria
- Short-circuit operators do not eagerly evaluate RHS.
- `in` and `..` support baseline literal/list/range cases.
- Concatenation operators behave deterministically for supported types.

## Verification
- `cargo test`
- Add tests under `tests/check_dump_ast_expressions.rs` and new run smoke cases.

## Suggested Commit
`feat(parity): add logical and collection operators`
