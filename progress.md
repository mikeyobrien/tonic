# Progress

## Current step
Builder implemented the plain text-block slice: `~t"""..."""` now lexes as a normalized plain string with framing-newline trim, common-indent dedent, raw heredoc preservation, and sharp rejection of unsupported `#{}` interpolation in this slice. The review target is current HEAD (this handoff commit) atop feature commit `727f507` plus harness-path repair commit `385d6b8`, so the shared working files now match the reviewed tree.

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
| The harness expected `.miniloop/context.md`, `.miniloop/plan.md`, `.miniloop/progress.md`, and `.miniloop/logs/`, but only root working files existed, so critic review hit ENOENT on `.miniloop/progress.md` | fix-now | Added `.miniloop/` symlinks pointing at the committed root working files and `logs/` so later roles can read the shared state from the documented path. |
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
- `.miniloop/context.md` (symlink)
- `.miniloop/plan.md` (symlink)
- `.miniloop/progress.md` (symlink)
- `.miniloop/logs` (symlink)

## Verification results
- `logs/text_block_lexer.log` — pass (`mkdir -p logs && cargo test text_block -- --nocapture > logs/text_block_lexer.log 2>&1`)
- `logs/text_block_heredoc_regression.log` — pass (`mkdir -p logs && cargo test heredoc -- --nocapture > logs/text_block_heredoc_regression.log 2>&1`)
- `logs/text_block_ast.log` — pass (`mkdir -p logs && cargo test --test check_dump_ast_expressions check_dump_ast_matches_text_block_literal_contract -- --exact > logs/text_block_ast.log 2>&1`)
- `logs/text_block_runtime.log` — pass (`mkdir -p logs && cargo test --test run_primitives_smoke run_executes_text_block_literals_with_trimmed_dedent_contract -- --exact > logs/text_block_runtime.log 2>&1`)
- `logs/text_block_manual_smoke.log` — pass (`mkdir -p logs && tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/examples" && printf 'defmodule Demo do\n  def run() do\n    ~t"""\n      hello\n        world\n\n      done\n    """\n  end\nend\n' > "$tmpdir/examples/text_block_manual.tn" && cargo run --bin tonic -- run "$tmpdir/examples/text_block_manual.tn" > logs/text_block_manual_smoke.log 2>&1 && rm -rf "$tmpdir"`)
- `logs/text_block_shared_files.log` — pass (`mkdir -p logs && ls -l .miniloop > logs/text_block_shared_files.log 2>&1 && test -L .miniloop/context.md && test -L .miniloop/plan.md && test -L .miniloop/progress.md && test -L .miniloop/logs`)

## Commits
- current HEAD — builder handoff commit for committed `plan.md` / `progress.md` updates (**review this tree**)
- `385d6b8` — `chore(miniloop): restore shared working file paths`
- `727f507` — `feat(lexer): add dedented text-block sigil`

## Handoff note for critic
Please review current HEAD (builder handoff commit above), not the feature commit in isolation. Independently smoke the live path with `cargo run --bin tonic -- run <fixture>` on a fixture containing `~t"""..."""`, and use `.miniloop/progress.md` as the authoritative handoff file now that the committed shared-file mirrors are in place.
