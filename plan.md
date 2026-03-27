# Plan

## Active slice
Wadler-Lindig algebra engine foundation in `src/formatter/algebra.rs`, isolated from the live formatter path.

## Why this slice now
The code task explicitly calls for an algebra engine before AST-to-doc conversion. Keeping it standalone makes the verification honest: the new code is exercised by focused algebra tests, while the unchanged runtime path is covered separately by regression tests.

## Builder checklist
- [x] Add `mod algebra;` in `src/formatter/mod.rs`.
- [x] Create `src/formatter/algebra.rs`.
- [x] Implement `Doc` with the task-listed variants: `Nil`, `Concat(Box<Doc>, Box<Doc>)`, `Nest(i32, Box<Doc>)`, `Text(String)`, `Line`, `Group(Box<Doc>)`, `FlexBreak(Box<Doc>)`.
- [x] Implement `format(doc: &Doc, max_width: usize) -> String`.
- [x] Keep semantics minimal and explicit:
  - `Group` tries flat layout first and falls back to broken layout when it does not fit.
  - `Line` renders as a space in flat mode and as `\n` plus current indentation in broken mode.
  - `Nest` increases indentation for broken lines only.
  - `FlexBreak` is re-evaluated inside broken layouts and can stay flat when the remaining suffix fits.
- [x] Add focused unit tests in `src/formatter/algebra.rs` covering:
  - flat group when content fits
  - broken group when width is exceeded
  - nested indentation after a broken line
  - concat / nil composition stability
  - `FlexBreak` partial reflow behavior with exact expected strings
- [x] Do **not** switch `format_source` to use the algebra engine yet.
- [x] Do **not** add AST-to-doc conversion, parser threading, config files, or CLI flags in this slice.
- [x] Run focused verification, save outputs under `logs/`, and commit only the slice files once green.

## Test plan
1. **Flat group stays flat**
   - Build a small grouped doc that fits within `max_width`.
   - Expect a single-line rendering with spaces instead of line breaks.
2. **Broken group wraps when too wide**
   - Use the same structure with a narrower width.
   - Expect deterministic line breaks and stable indentation.
3. **Nested indentation**
   - Use `Nest` around a broken inner group.
   - Expect continuation lines to pick up the nested indentation.
4. **FlexBreak behavior**
   - Build a grouped doc where the outer group must break, but a later flex break can stay inline.
   - Expect a mixed rendering that proves `FlexBreak` is not identical to `Line`.
5. **No live formatter regression**
   - Re-run one existing formatter regression test and one CLI smoke/parity test to prove the runtime path still works unchanged.

## Verification commands
- `cargo test formatter::algebra -- --nocapture`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture`

## Critic note
There is still no honest manual smoke for the changed code path because `tonic fmt` remains wired to `src/formatter/engine.rs`. The critic should treat any CLI/manual formatter run as regression evidence only, not as proof that the new algebra module executed.
