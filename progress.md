# Progress

## Current step
Builder implemented slice 5 from baseline commit `44cd087`: `src/formatter/to_doc.rs` now renders AST `case` expressions, branch guards, and the current parser-covered pattern family so lowered control forms format on the library-only path.

## Next role
critic

## Prior slices
- Slice 1 comment preservation landed on `e81e939`.
- Slice 2 algebra engine landed on `aee0387`.
- Slice 3 AST-to-doc baseline landed on `d790d6c`.
- Slice 4 collection/data literal + `defstruct` AST rendering landed on `44cd087`.

## Active slice acceptance
- [x] Render `Expr::Case` on the AST path.
- [x] Render branch guards as `when` clauses.
- [x] Render wildcard, bind, pin, literal, tuple, list, map, and struct patterns needed by current parser coverage.
- [x] Prove lowered `if` / `unless` / `cond` / `with` render deterministically through focused parse -> doc -> render tests.
- [x] Keep live `tonic fmt` on `src/formatter/engine.rs`.
- [x] Re-run live-path regression tests so unchanged formatter behavior stays covered.
- [x] Commit only slice-relevant files plus shared task docs/logs before `review.ready`.

## Builder guardrails
- Re-read the shared files and touched source before editing.
- Do not broaden the slice into module attributes/forms, comment reinsertion, `try`/`raise`, interpolation, anonymous functions, bitstrings, config, or runtime wiring.
- If a helper in `src/formatter/to_doc.rs` becomes generic enough to simplify existing collection/call formatting, refactor it only if the focused tests stay exact and local.
- There are unrelated dirty files in the repo; do not stage them.
- Cite the new commit hash in the `review.ready` handoff.

## Relevant Issues
| Issue | Disposition | Notes |
|---|---|---|
| AST formatter lacks `case` and pattern rendering, so direct `case` plus lowered `if`/`unless`/`cond`/`with` cannot render | fix-now | This is the concrete slice-5 objective |
| AST formatter path still is not wired into live `tonic fmt` | fix-next | Keep runtime routing unchanged until the AST path covers more core expression families |
| AST comment reinsertion still does not exist | deferred | Keep separate until AST coverage is much wider and closer to live use |
| Module attributes plus alias/import/require/use/protocol/defimpl forms are still unsupported on the AST path | deferred | Different surface; do not mix with case/pattern work |
| `try`/`rescue`/`catch`/`after`, `raise`, `for`, anonymous functions, interpolation, and bitstrings are still unsupported on the AST path | deferred | Handle in later focused slices |
| Manual smoke cannot honestly exercise `src/formatter/to_doc.rs` yet | out-of-scope | Critic should use targeted cargo tests for proof and treat CLI runs only as regression coverage |
| Repo has unrelated dirty changes outside formatter surface | out-of-scope | Commit only formatter-slice files plus shared task docs/logs |

## Verification plan
- `cargo test formatter::to_doc -- --nocapture > logs/formatter_to_doc_control.log 2>&1`
- `cargo test formatter::algebra -- --nocapture > logs/formatter_algebra.log 2>&1`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture > logs/formatter_regression.log 2>&1`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture > logs/formatter_parity.log 2>&1`

## Builder implementation notes
- Added `Expr::Case` rendering to `expr_to_doc` with dedicated case/branch helpers and stable indentation for nested case bodies.
- Added pattern rendering helpers for wildcard, bind, pin, literal, tuple, list, list-cons tail, map, and struct patterns.
- Added focused AST formatter tests covering direct `case`, nested case bodies, and parser-lowered `if` / `unless` / `cond` / `with` output.
- Left `src/formatter/mod.rs::format_source` untouched on the token formatter path; there is still no honest manual smoke path for the new AST-only code.

## Verification results
- `logs/formatter_to_doc_control.log` — pass (`cargo test formatter::to_doc -- --nocapture`)
- `logs/formatter_algebra.log` — pass (`cargo test formatter::algebra -- --nocapture`)
- `logs/formatter_regression.log` — pass (`cargo test format_source_is_idempotent_with_comments -- --nocapture`)
- `logs/formatter_parity.log` — pass (`cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture`)

## Commit
- Slice committed with `feat(formatter): render AST case control forms`; cite the final hash in the builder handoff.

## Planner notes
- Slice 5 stays library-only by design; there is still no honest CLI/manual smoke path into the new formatter code while `format_source` remains token-driven.
- The most leverage comes from `Expr::Case` because parser lowering already routes `if`, `unless`, `cond`, and `with` through that AST family.
- Builder should land the slice as a single commit before handing off.

## Handoff expectation for builder
Implement the slice above, save the verification outputs under the listed log files, and hand off with exact files changed, exact commands run, and the new commit hash.
