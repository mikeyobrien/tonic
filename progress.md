# Progress

## Current step
Builder completed the interpolation follow-up on top of baseline `13962bc`. `~t"""..."""` now supports `#{...}` while keeping the existing framing-newline trim and common-indent dedent contract for plain text blocks and leaving raw heredocs unchanged.

## Next role
critic

## Allowed next event
review.passed, review.rejected

## Slice status
### Baseline already landed at `13962bc`
- [x] Plain `~t"""..."""` lexer support
- [x] Centralized framing-newline trim + common-indent dedent helper
- [x] Plain text-block lexer / AST / runtime coverage
- [x] Raw heredoc preservation
- [x] Shared `.miniloop/*` path repair

### Interpolation follow-up in this slice
- [x] `#{}` interpolation inside text blocks
- [x] Dedent defined over logical output lines so multiline interpolation source indentation does not distort surrounding text
- [x] Positive lexer coverage for interpolated text blocks, including interpolation on an otherwise blank content line
- [x] Malformed interpolation diagnostic coverage for missing `}` inside `~t"""`
- [x] AST dump regression for interpolated text blocks
- [x] Runtime regression for interpolated text blocks with multiline interpolation expressions
- [x] `TONIC_REFERENCE.md` updated to document interpolation semantics

## Relevant issues
| Issue | Disposition | Notes |
|---|---|---|
| Original task requires interpolation inside `~t"""..."""`, and baseline `13962bc` intentionally deferred it | fix-now | Fixed in this slice by normalizing logical text-block lines and then reusing the existing interpolated-string token flow. |
| Current docs still said text-block interpolation was unsupported | fix-now | Fixed in `TONIC_REFERENCE.md`. |
| Current lexer tests included a rejection path for text-block interpolation | fix-now | Replaced with positive token-flow coverage plus a malformed interpolation diagnostic test. |
| Worktree contains many unrelated edits outside this loop | out-of-scope | Review only the files listed below; do not stage or rely on unrelated dirty paths. |
| Formatter support for `~t` remains tempting follow-up work | deferred | Still intentionally out of scope for this slice. |

## Files changed in this slice
- `src/lexer/string_scan.rs`
- `src/lexer/tests.rs`
- `tests/check_dump_ast_string_interpolation.rs`
- `tests/run_primitives_smoke.rs`
- `TONIC_REFERENCE.md`
- `plan.md`
- `progress.md`

## Verification run for review target current HEAD (`feat(lexer): support interpolation in text blocks`)
- `mkdir -p logs && cargo test text_block -- --nocapture > logs/text_block_lexer.log 2>&1`
- `mkdir -p logs && cargo test --test check_dump_ast_string_interpolation -- --nocapture > logs/text_block_ast_interpolation.log 2>&1`
- `mkdir -p logs && cargo test --test run_primitives_smoke text_block -- --nocapture > logs/text_block_runtime_interpolation.log 2>&1`
- `mkdir -p logs && cargo test heredoc -- --nocapture > logs/text_block_heredoc_regression.log 2>&1`
- manual smoke: `cargo run --bin tonic -- run /var/folders/s9/x5s6jsl12p3f371gpx8cw7cc0000gn/T/tmp.GbDkz0RlhA/examples/text_block_manual.tn`

## Verification artifacts
- `logs/text_block_lexer.log`
- `logs/text_block_ast_interpolation.log`
- `logs/text_block_runtime_interpolation.log`
- `logs/text_block_heredoc_regression.log`
- `logs/text_block_manual_interpolation.log`
- `logs/text_block_manual_fixture_path.txt`

## Handoff note for critic
Review current HEAD. The changed path is entirely in text-block lexing: the builder now buffers `~t` fragments, computes dedent over logical output lines, and emits the existing interpolated-string token sequence with original-expression spans shifted back into the source file. Independently rerun the listed verification and smoke an interpolated fixture with `cargo run --bin tonic -- run <fixture>` before passing.
