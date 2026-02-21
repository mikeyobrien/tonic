# Task: Pattern Pin + Guards + Match Operator

## Goal
Add the core pattern tools used in real Elixir code: `^pin`, guards, and `=` matching.

## Scope
- Parser and AST support for pin patterns (`^var`).
- Guard clauses (`when`) in `case` branches and function clauses.
- Match operator (`=`) semantics for binding/destructuring expressions.
- Type/runtime diagnostics for guard failures and bad matches.

## Out of Scope
- Full guard BIF parity; implement minimal guard predicate subset first.

## Deliverables
- End-to-end parse/lower/runtime support for pin/guards/match.
- Compatibility tests for common destructuring and guard patterns.

## Acceptance Criteria
- Pinned values are enforced during pattern matching.
- Guarded clauses select/fall through correctly.
- `=` bindings support destructuring and mismatch diagnostics.

## Verification
- `cargo test`
- Add dedicated tests in parser/typing/runtime + integration smoke tests.

## Suggested Commit
`feat(parity): add pin patterns guards and match operator`
