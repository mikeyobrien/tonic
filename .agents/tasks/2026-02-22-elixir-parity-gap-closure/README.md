# Elixir Parity Gap Closure — Sequenced Tasks

Source checklist: `research/elixir-syntax-parity-checklist.md`

Goal: close the highest-impact remaining syntax/semantics gaps with small, test-backed commits.

## Sequence

1. `01-string-interpolation.code-task.md` — done
2. `02-map-update-access.code-task.md` — done
3. `03-comprehensions-for.code-task.md` — pending
4. `04-try-rescue-raise.code-task.md` — pending

## Done Definition (per task)

- Tests added/updated for parser + runtime behavior.
- `cargo fmt --all` passes.
- `cargo clippy --all-targets --all-features -- -D warnings -A dead_code` passes.
- `cargo test` passes.
- Checklist status updated in `research/elixir-syntax-parity-checklist.md`.
