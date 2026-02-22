---
status: done
HEARTBEAT_TASK_STATUS: done
---

# Task: String Interpolation (`"#{expr}"`)

## Goal
Add baseline string interpolation support so expressions can be embedded inside string literals.

## Scope
- Lexer support for interpolation boundaries in string literals.
- Parser/AST representation for interpolated string segments.
- IR lowering and runtime evaluation for mixed literal/expression segments.
- Rendering compatible with existing `tonic run` output style.

## Out of Scope
- Sigils.
- Advanced escaping parity beyond interpolation.

## Deliverables
- End-to-end `"#{...}"` support for simple expressions.
- Deterministic diagnostics for malformed interpolation syntax.
- Tests for dump-ast, dump-ir, and run output contracts.

## Acceptance Criteria
- `"hello #{1 + 2}"` evaluates to `"hello 3"`.
- Multiple interpolation segments evaluate in-order.
- Invalid interpolation emits deterministic parser diagnostics.

## Verification
- `cargo test`
- Add/extend tests in lexer/parser + integration smoke tests.

## Suggested Commit
`feat(parity): add baseline string interpolation support`
