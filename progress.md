# Progress

## Current step
Implemented slice 3: added a library-only AST-to-doc formatter in `src/formatter/to_doc.rs`, wired it into `src/formatter/mod.rs` without changing the live CLI path, and extended `src/formatter/algebra.rs` with `SoftLine` so flat vs broken call layouts render cleanly.

## Next role
critic

## Active slice acceptance
- [x] Create `src/formatter/to_doc.rs` and wire it into `src/formatter/mod.rs` as a non-live module.
- [x] Convert a narrow AST surface into algebra docs: modules, `def`/`defp`, identifier params/defaults, guards, simple calls, pipe chains, and block bodies.
- [x] Prove width-sensitive layout with focused parse -> doc -> render tests.
- [x] Keep `tonic fmt` on the existing token formatter path.
- [x] Re-run live-path regression tests so unchanged formatter behavior stays covered.

## Builder result
Changed files:
- `src/formatter/mod.rs`
- `src/formatter/algebra.rs`
- `src/formatter/to_doc.rs`
- `context.md`
- `plan.md`
- `progress.md`

Notably unchanged on purpose:
- live `format_source` / `tonic fmt` routing still goes through `src/formatter/engine.rs`
- no CLI/config work
- no AST comment reinsertion
- no broad control-flow/maps/structs coverage yet

## Relevant Issues
| Issue | Disposition | Notes |
|---|---|---|
| AST formatter path still is not wired into live `tonic fmt` | fix-next | This slice proved AST -> doc conversion without changing runtime routing |
| AST formatter does not yet cover module forms, attributes, maps/structs, or control-flow forms | fix-next | `src/formatter/to_doc.rs` currently errors on unsupported nodes outside the narrow slice-3 surface |
| Non-trivial function-head pattern rendering is still missing on the AST path | deferred | Identifier params/defaults/guards only in this slice |
| AST comment reinsertion still does not exist | deferred | Keep this separate until the AST formatter path is live enough to justify it |
| `--line-length` / formatter config support is still absent | deferred | Wait until runtime wiring lands |
| `cargo run -- fmt <path>` is ambiguous in this workspace; manual smoke needs `cargo run --bin tonic -- fmt <path>` | deferred | Still relevant for later live-path slices, but not proof for this library-only slice |
| Repo has unrelated dirty changes outside formatter surface | out-of-scope | Commit only formatter-slice files plus shared task docs |
| Manual smoke cannot honestly exercise `src/formatter/to_doc.rs` yet | out-of-scope | Critic should use targeted cargo tests for proof and treat CLI runs only as regression coverage |

## Status table
| Surface | Status | Notes |
|---|---|---|
| Slice 1 comment preservation | done | Landed on `e81e939` |
| Slice 2 algebra engine | done | Landed on `aee0387`; touched here only to add `SoftLine` for AST call layout |
| Slice 3 AST-to-doc converter | done | Ready for review in this iteration |
| Live CLI/runtime formatter switch | todo | Not part of this iteration |
| CLI/config line-length support | todo | Not part of this iteration |

## Verification evidence
Commands run:
- `cargo test formatter::to_doc -- --nocapture`
- `cargo test formatter::algebra -- --nocapture`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture`

Logs:
- `logs/formatter_to_doc.log`
- `logs/formatter_algebra.log`
- `logs/formatter_regression.log`
- `logs/formatter_parity.log`

## Critic guidance
- Review `src/formatter/to_doc.rs`, `src/formatter/algebra.rs`, and `src/formatter/mod.rs`.
- Confirm the new tests cover modules/functions/defaults/guards/calls/pipes/blocks and that unsupported surfaces are still intentionally excluded.
- There is still no honest manual smoke for the new AST path because `tonic fmt` does not call it yet. If you run CLI formatting, treat it as regression evidence only.
- If you pass the slice, cite the exact files above plus the four verification commands/logs.

## Commit status
Committed slice 3 in current `HEAD` with:
- `src/formatter/mod.rs`
- `src/formatter/algebra.rs`
- `src/formatter/to_doc.rs`
- `context.md`
- `plan.md`
- `progress.md`
- `logs/formatter_to_doc.log`
- `logs/formatter_algebra.log`
- `logs/formatter_regression.log`
- `logs/formatter_parity.log`
