# Progress

## Current step
Builder implemented the plain text-block slice: `~t"""..."""` now lexes as a normalized plain string with framing-newline trim, common-indent dedent, raw heredoc preservation, and sharp rejection of unsupported `#{}` interpolation in this slice.

## Next role
critic

## Active slice checklist
- [x] Add plain `~t"""..."""` lexer support
- [x] Centralize trim/dedent normalization in one helper
- [x] Add lexer edge-case coverage
- [x] Add AST dump regression
- [x] Add runtime regression
- [x] Preserve raw heredoc behavior
- [x] Update `TONIC_REFERENCE.md`
- [x] Save logs under `logs/`
- [x] Commit before `review.ready`

## Relevant Issues
| Issue | Disposition | Notes |
|---|---|---|
| Prompt says `.agents/planning`, but the actionable task lives at `.agents/tasks/2026-03-27-text-block-ergonomics/dedented-text-block-sigil.code-task.md` | out-of-scope | Used the task file as the implementation source of truth. |
| The worktree already contains many unrelated modified/deleted files | out-of-scope | Kept edits scoped to text-block files plus shared working files; do not stage unrelated paths. |
| `scan_sigil` only understood `~s`, `~r`, and `~w`, so `~t` needed a dedicated recognition/error path | fix-now | Added dedicated `~t"""..."""` handling and explicit rejection of unsupported `~t(...)` spellings. |
| Interpolated text blocks need a more invasive lexer design because dedent must coexist with the interpolation token stream | fix-next | This slice rejects `~t"""...#{...}..."""` sharply; add interpolation-aware normalization in the follow-up slice. |
| Self-hosted lexer parity does not yet cover the new syntax | deferred | Keep the Rust lexer path proven first; parity follow-up stays separate. |

## Files changed
- `src/lexer/string_scan.rs`
- `src/lexer/tests.rs`
- `tests/check_dump_ast_expressions.rs`
- `tests/run_primitives_smoke.rs`
- `TONIC_REFERENCE.md`
- `context.md`
- `plan.md`
- `progress.md`

## Verification results
- `logs/text_block_lexer.log` — pass (`mkdir -p logs && cargo test text_block -- --nocapture > logs/text_block_lexer.log 2>&1`)
- `logs/text_block_heredoc_regression.log` — pass (`mkdir -p logs && cargo test heredoc -- --nocapture > logs/text_block_heredoc_regression.log 2>&1`)
- `logs/text_block_ast.log` — pass (`mkdir -p logs && cargo test --test check_dump_ast_expressions check_dump_ast_matches_text_block_literal_contract -- --exact > logs/text_block_ast.log 2>&1`)
- `logs/text_block_runtime.log` — pass (`mkdir -p logs && cargo test --test run_primitives_smoke run_executes_text_block_literals_with_trimmed_dedent_contract -- --exact > logs/text_block_runtime.log 2>&1`)

## Commit
- `f5891bc` — `feat(lexer): add dedented text-block sigil`

## Handoff note for critic
Please independently smoke the live path with `cargo run --bin tonic -- run <fixture>` on a fixture containing `~t"""..."""`; the builder only ran the required cargo-test evidence above.
