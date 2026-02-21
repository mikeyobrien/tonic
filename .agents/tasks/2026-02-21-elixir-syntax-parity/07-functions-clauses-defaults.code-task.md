# Task: Function Parity (clauses/defaults/defp)

## Goal
Bring named function syntax closer to Elixir with clauses, head patterns, defaults, and private defs.

## Scope
- Multi-clause function definitions (same name/arity).
- Pattern matching in function heads.
- Guard clauses in function heads.
- Default args (`\\`).
- `defp` private functions with visibility enforcement.

## Out of Scope
- Anonymous function syntax (next task).

## Deliverables
- Function dispatch model in resolver/lowering/runtime.
- Deterministic arity/visibility diagnostics.

## Acceptance Criteria
- Clause selection works by pattern/guard order.
- Defaults resolve to expected arity variants.
- Calls to `defp` across module boundaries fail deterministically.

## Verification
- `cargo test`
- Add focused integration tests for dispatch/default/visibility cases.

## Suggested Commit
`feat(parity): add function clauses defaults and defp visibility`
