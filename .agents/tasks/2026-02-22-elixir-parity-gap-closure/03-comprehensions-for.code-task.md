---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Comprehensions (`for`)

## Goal
Add baseline `for` comprehension support for list-based transformations.

## Scope
- Lexer/parser support for `for` and `<-` comprehension syntax.
- Baseline semantics: single generator + expression body.
- Runtime evaluation producing list results.

## Out of Scope
- Multi-generator comprehensions.
- `into:` and advanced comprehension options.
- Bitstring comprehensions.

## Deliverables
- End-to-end `for` support for baseline form.
- Deterministic diagnostics for unsupported comprehension options.
- Tests for parser + runtime behavior.

## Acceptance Criteria
- `for x <- [1,2,3] do x + 1 end` evaluates to `[2, 3, 4]`.
- Unsupported forms fail with deterministic errors.

## Verification
- `cargo test`
- Add dedicated dump-ast + run smoke tests.

## Suggested Commit
`feat(parity): add baseline for comprehension support`
