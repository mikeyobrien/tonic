# Elixir Syntax Parity — Sequenced Repo Tasks

Source checklist: `research/elixir-syntax-parity-checklist.md`

Goal: deliver syntax/semantics parity in small, reviewable commits with deterministic tests.

## Sequence

1. `01-literals-primitives.code-task.md`
2. `02-operators-arith-compare.code-task.md`
3. `03-operators-logic-collection.code-task.md`
4. `04-literal-collections.code-task.md`
5. `05-pattern-runtime-list-map.code-task.md`
6. `06-pattern-pin-guards-match.code-task.md`
7. `07-functions-clauses-defaults.code-task.md`
8. `08-anon-fn-capture.code-task.md`
9. `09-control-if-cond-with.code-task.md`
10. `10-module-forms-attrs.code-task.md`
11. `11-tooling-parity-sweep.code-task.md`

## Done Definition (per task)

- Tests added/updated for new syntax + runtime behavior.
- `cargo test` passes.
- `cargo fmt --all` clean.
- `cargo clippy --all-targets --all-features -- -D warnings -A dead_code` passes.
- One conventional commit per task.
