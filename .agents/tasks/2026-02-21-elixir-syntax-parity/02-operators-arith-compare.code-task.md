# Task: Operators Arithmetic + Comparison

## Goal
Implement core arithmetic and comparison operators with deterministic precedence.

## Scope
- Operators: `-`, `*`, `/`, `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Parser precedence table updates.
- IR ops and runtime execution.
- Type-checking rules and diagnostics for operator misuse.

## Out of Scope
- `and/or/not`, `&&/||/!`.
- `<>`, `++`, `--`, `in`, `..`.

## Deliverables
- Extended lexer/parser/IR/runtime operator support.
- Golden tests for precedence and dump snapshots.

## Acceptance Criteria
- Mixed expressions parse and evaluate with correct precedence.
- Invalid operand types produce stable type/runtime diagnostics.

## Verification
- `cargo test`
- New tests: `tests/check_dump_ast_expressions.rs`, `tests/run_arithmetic_smoke.rs`.

## Suggested Commit
`feat(parity): add arithmetic and comparison operators`
