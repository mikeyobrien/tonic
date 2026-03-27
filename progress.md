# Progress

## Current step
Builder implementation complete for slice 2: the standalone Wadler-Lindig algebra engine is in place, focused verification is green, and the live formatter path remains intentionally unchanged.

## Next role
critic

## Active slice acceptance
- Add `src/formatter/algebra.rs` with the task-required `Doc` variants.
- Implement `format(doc, max_width)` with tested flat/broken group behavior.
- Prove `Nest` indentation and `FlexBreak` semantics with focused unit tests.
- Keep `tonic fmt` on the existing token formatter path for now.
- Re-run at least one existing formatter regression path to show no behavior regression in the live CLI path.

## Relevant Issues
| Issue | Disposition | Notes |
|---|---|---|
| New algebra engine is not wired into the live formatter path yet | fix-next | Intentional for this slice; next formatter slices should build AST-to-doc conversion and then switch `format_source` over deliberately |
| Formatter is still token-driven and not line-length aware | fix-next | The algebra engine is now the prerequisite, but runtime behavior is unchanged |
| No AST-to-doc conversion exists yet | fix-next | Start only after this algebra slice is reviewed and committed |
| No `--line-length` / formatter config support | deferred | Wait until AST-driven formatting exists and width handling is live |
| `cargo run -- fmt <path>` is ambiguous in this workspace; manual smoke needs `cargo run --bin tonic -- fmt <path>` | deferred | Relevant for future live formatter slices, but this slice has no honest manual path into `src/formatter/algebra.rs` |
| Repo has unrelated dirty changes outside formatter surface | out-of-scope | Do not modify or stage unrelated files; commit only formatter slice files plus shared task docs |

## Status table
| Surface | Status | Notes |
|---|---|---|
| Slice 1 comment preservation | done | Landed on `e81e939` |
| `src/formatter/algebra.rs` | done | Added standalone algebra engine and focused tests |
| AST-to-doc conversion | todo | Explicitly not part of this iteration |
| CLI/config line-length support | todo | Explicitly not part of this iteration |

## Files changed in this slice
- `src/formatter/algebra.rs`
- `src/formatter/mod.rs`
- `context.md`
- `plan.md`
- `progress.md`

## Verification
- `cargo test formatter::algebra -- --nocapture`
  - log: `logs/formatter_algebra.log`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture`
  - log: `logs/formatter_regression.log`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture`
  - log: `logs/formatter_parity.log`

## Builder notes
- The new algebra module is intentionally standalone and not re-routed into `format_source` yet.
- `FlexBreak` is implemented as a re-evaluated subdocument: in a broken layout it still probes the remaining suffix and may render flat.
- Regression coverage proves the live formatter path still behaves as before, but it does **not** exercise the new algebra code path.

## Commit
- Landed at current `HEAD` as `feat(formatter): add standalone algebra engine`
