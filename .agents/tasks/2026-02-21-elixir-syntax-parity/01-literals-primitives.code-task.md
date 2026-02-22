# Task: Literals Primitives (bool/nil/string)

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Add first-class expression support for `true`, `false`, `nil`, and string literals.

## Scope
- Lexer keywords/tokens for bool/nil (if needed).
- Parser `Expr` support for bool/nil/string.
- IR ops for bool/nil/string constants.
- Runtime values/rendering/evaluation support.
- Type inference baseline labels for new primitive types.

## Out of Scope
- String interpolation.
- Heredocs/sigils/bitstrings.

## Deliverables
- New AST/IR/runtime variants for primitive literals.
- Integration tests for `tonic run` and `tonic check --dump-ast/--dump-ir`.

## Acceptance Criteria
- `tonic run` prints expected values for primitive literal programs.
- `check --dump-ast`/`--dump-ir` include stable shapes for new literals.

## Verification
- `cargo test`
- Add targeted tests in `tests/check_dump_ast_expressions.rs` and `tests/run_*`.

## Suggested Commit
`feat(parity): add bool nil and string literal expressions`
