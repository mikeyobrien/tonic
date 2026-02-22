# Task: Collection Literal Syntax

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Goal
Support Elixir-style literal syntax for tuples, lists, maps, and keywords.

## Scope
- Literal forms: `{a, b}`, `[a, b]`, `%{k: v}`, `[k: v]`.
- Parser AST nodes for collection literals.
- IR lowering and runtime construction semantics.
- Render consistency with existing output contracts.

## Out of Scope
- Map update syntax `%{m | k: v}`.
- Full access syntax (`map.key`, `map[:key]`).

## Deliverables
- Syntax support replacing constructor-only ergonomics.
- Tests that compare literal forms with existing builtin constructor behavior.

## Acceptance Criteria
- Literal programs run successfully and render expected results.
- `check --dump-ast` shows stable literal node contracts.

## Verification
- `cargo test`
- Add/extend tests: `tests/run_collections_smoke.rs`, AST/IR dump snapshots.

## Suggested Commit
`feat(parity): add tuple list map and keyword literal syntax`
