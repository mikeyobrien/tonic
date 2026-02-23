---
status: done
HEARTBEAT_TASK_STATUS: done
---

# Task: Map Update + Access Syntax (`%{m | k: v}`, `map.key`, `map[:key]`)

## Goal
Add missing map update and access forms needed for idiomatic Elixir-style data manipulation.

## Scope
- Parser support for map-update literal form `%{base | key: value}`.
- Parser + lowering/runtime for access forms:
  - `map.key`
  - `map[:key]`
- Deterministic runtime/typing diagnostics for invalid access/update cases.

## Out of Scope
- Full struct update semantics.
- Nested access rewrite macros.

## Deliverables
- AST/IR/runtime support for map update and access forms.
- Tests for positive and negative paths in check/run flows.

## Acceptance Criteria
- `%{m | done: true}` updates an existing map value deterministically.
- `map.key` and `map[:key]` return expected values for supported key types.
- Invalid base/access types produce deterministic diagnostics.

## Verification
- `cargo test`
- Add/extend tests in dump-ast, dump-ir, run smoke, and typing diagnostics.

## Suggested Commit
`feat(parity): add map update and access syntax`
