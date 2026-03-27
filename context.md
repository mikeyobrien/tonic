# Context

## Objective
Implement `code-task.formatter.md` incrementally. Slice 1 landed on `e81e939` and fixed comment preservation in the existing token formatter. Slice 2 now adds the standalone algebra foundation needed for the later AST-driven formatter work.

## Current repo state
- `tonic fmt` still routes through `src/formatter/mod.rs` -> `engine::format_source_inner`.
- `src/formatter/algebra.rs` now exists as a standalone Wadler-Lindig-style document engine.
- No AST-to-doc converter exists yet.
- No width-aware wrapping or `--line-length` CLI support exists yet.
- Comment preservation from slice 1 remains unchanged and green.

## Slice 2 outcome
- Added `src/formatter/algebra.rs` with `Doc` variants: `Nil`, `Concat`, `Nest`, `Text`, `Line`, `Group`, `FlexBreak`.
- Implemented `format(doc, max_width)` with flat-vs-broken group decisions.
- Implemented `Nest` indentation handling for broken layouts.
- Implemented `FlexBreak` as a re-evaluated break that can stay inline inside an already-broken group when the remaining suffix still fits.
- Kept the live formatter path unchanged; `tonic fmt` still uses `src/formatter/engine.rs`.

## Relevant code
- `src/formatter/mod.rs` — module wiring; `format_source` still points at `engine`.
- `src/formatter/engine.rs` — current live formatter path.
- `src/formatter/algebra.rs` — new standalone algebra engine plus focused unit tests.
- `code-task.formatter.md` — source task; this slice covers Task 2 only.
- `tests/fmt_parity_smoke.rs` and formatter unit tests — regression proof that the live path stayed intact.

## Constraints still in force
- Keep exactly one concrete slice active.
- Do not mix in AST-to-doc conversion, parser threading, config loading, or `--line-length` CLI wiring yet.
- Because the new module is not on the live CLI path, a CLI/manual smoke can only count as regression coverage for the existing formatter path, not proof that `src/formatter/algebra.rs` was exercised.
- Only stage formatter slice files plus the shared planning docs for this slice.

## Verification evidence
- `logs/formatter_algebra.log` — focused algebra tests
- `logs/formatter_regression.log` — existing formatter idempotency regression
- `logs/formatter_parity.log` — existing CLI parity smoke regression
