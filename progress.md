# Progress

## Current step
Builder completed slice 4 in the working tree: `src/formatter/to_doc.rs` now renders `defstruct`, tuple/list/keyword/map/struct literals, and map/struct updates on the AST path while leaving live `tonic fmt` routing unchanged.

## Next role
critic

## Prior slices
- Slice 1 comment preservation landed on `e81e939`.
- Slice 2 algebra engine landed on `aee0387`.
- Slice 3 AST-to-doc baseline landed on `d790d6c`.

## Active slice acceptance
- [x] Render `defstruct` module forms on the AST path.
- [x] Preserve a stable blank line between module forms and sibling functions.
- [x] Render tuple and list literals.
- [x] Render keyword and map literals.
- [x] Render struct literals plus map/struct updates.
- [x] Prove flat-vs-broken collection layout with focused parse -> doc -> render tests.
- [x] Keep live `tonic fmt` on `src/formatter/engine.rs`.
- [x] Re-run live-path regression tests so unchanged formatter behavior stays covered.
- [x] Commit only slice-relevant files plus shared task docs/logs.

## Builder guardrails
- Re-read the shared files and touched source before editing.
- Do not broaden the slice into module attributes, protocol forms, comment reinsertion, config, or runtime wiring.
- If a helper in `src/formatter/to_doc.rs` becomes generic enough to simplify existing call/pipe formatting, refactor it only if the focused tests stay exact and local.
- There are unrelated dirty files in the repo; do not stage them.

## Relevant Issues
| Issue | Disposition | Notes |
|---|---|---|
| AST formatter still rejects modules containing `defstruct` | fix-now | Needed to render realistic struct fixtures on the AST path |
| AST formatter lacks tuple/list/keyword/map/struct/update rendering | fix-now | This is the concrete slice-4 objective |
| AST formatter path still is not wired into live `tonic fmt` | fix-next | Keep runtime routing unchanged in this slice |
| AST comment reinsertion still does not exist | deferred | Keep separate until the AST formatter path is wider and closer to live use |
| Module attributes plus alias/import/require/use/protocol/defimpl forms are still unsupported on the AST path | deferred | Different surface; do not mix with collection literal work |
| Broader control-flow families (`case`, `try`, comprehensions, anonymous functions) still need AST rendering | deferred | Handle in later focused slices |
| `--line-length` / formatter config support is still absent | deferred | Wait until runtime wiring lands |
| Manual smoke cannot honestly exercise `src/formatter/to_doc.rs` yet | out-of-scope | Critic should use targeted cargo tests for proof and treat CLI runs only as regression coverage |
| Repo has unrelated dirty changes outside formatter surface | out-of-scope | Commit only formatter-slice files plus shared task docs/logs |

## Verification run
- `cargo test formatter::to_doc -- --nocapture > logs/formatter_to_doc_collections.log 2>&1` ✅
- `cargo test formatter::algebra -- --nocapture > logs/formatter_algebra.log 2>&1` ✅
- `cargo test format_source_is_idempotent_with_comments -- --nocapture > logs/formatter_regression.log 2>&1` ✅
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture > logs/formatter_parity.log 2>&1` ✅

## Builder notes
- Changed only `src/formatter/to_doc.rs`, `plan.md`, `progress.md`, and the four slice-4 log files.
- The new tests stay library-only; there is still no honest manual smoke path into `src/formatter/to_doc.rs` because `src/formatter/mod.rs::format_source` still calls `engine::format_source_inner`.
- Commit recorded for this slice; cite the exact hash in the `review.ready` handoff.

## Handoff expectation for critic
Critic should validate the reviewed working tree/commit from the log files above and keep treating CLI/manual formatter runs only as regression evidence, not proof that `src/formatter/to_doc.rs` executed.
