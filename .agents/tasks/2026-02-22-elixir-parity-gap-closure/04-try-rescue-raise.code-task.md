---
status: done
HEARTBEAT_TASK_STATUS: done
---

# Task: `try/rescue` + `raise` Baseline

## Goal
Add baseline exception-style control flow with deterministic failure handling.

## Scope
- Lexer/parser support for `try`, `rescue`, `raise`.
- Runtime semantics for raising and rescuing baseline errors.
- Deterministic diagnostics when no rescue matches.

## Out of Scope
- `catch` / `after` in full parity depth.
- Stack traces and advanced exception classes.

## Deliverables
- End-to-end `try/rescue` + `raise` baseline behavior.
- Tests for rescued and unrescued error flows.

## Acceptance Criteria
- `try do raise(:boom) rescue _ -> :ok end` returns `:ok`.
- Unrescued raise fails with deterministic runtime diagnostic.

## Verification
- `cargo test`
- Add parser and runtime integration tests.

## Suggested Commit
`feat(parity): add baseline try rescue and raise support`
