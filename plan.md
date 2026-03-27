# Plan

## Active slice
Create `src/formatter/to_doc.rs` as the first AST-to-algebra converter, kept completely off the live `tonic fmt` path.

## Why this slice now
Task 3 is the bulk of the formatter rewrite. The right next step is not CLI wiring; it is proving that parsed AST nodes can be rendered into the existing algebra with exact, testable output. Keep the supported AST surface intentionally small so failures stay local and the next slice can extend it cleanly.

## Builder checklist
- [x] Re-read `src/formatter/mod.rs`, `src/formatter/algebra.rs`, `src/parser/mod.rs`, and `src/parser/ast/{mod.rs,expr_def.rs,expr_impl.rs}` before editing; do not chase the nonexistent `src/parser.rs` path from the prior scratchpad.
- [x] Add `mod to_doc;` in `src/formatter/mod.rs`.
- [x] Create `src/formatter/to_doc.rs`.
- [x] Add a pure entrypoint that converts parsed AST/modules/functions into `algebra::Doc` and returns formatted text via `algebra::format` for tests.
- [x] Support module rendering:
  - [x] `defmodule <Name> do ... end`
  - [x] blank line separation between sibling functions
- [x] Support function rendering:
  - [x] `def` / `defp`
  - [x] identifier parameter lists
  - [x] default args (`\\`)
  - [x] optional guards (`when ...`)
- [x] Support only the expression subset needed by focused tests:
  - [x] variables and literals (`int`, `float`, `bool`, `nil`, `string`)
  - [x] simple calls / module-qualified calls represented by AST `Call`
  - [x] blocks (`Expr::Block`)
  - [x] pipes (`Expr::Pipe`) with one pipe per line when broken
- [x] Use the algebra engine for width-sensitive layout decisions in function calls and pipe chains.
- [x] Keep `format_source` and `tonic fmt` wired to `src/formatter/engine.rs` in this slice.
- [x] Do **not** implement comment reinsertion, parser threading into CLI, config loading, maps/structs, control-flow doc conversion, or non-trivial function-head pattern rendering yet.
- [x] Add focused unit tests that parse source, convert to docs, render with a chosen width, and assert exact strings.
- [x] Save test outputs under `logs/` and do not stage unrelated dirty files.

## Test plan
1. **Module + function shell**
   - Parse a module with sibling functions.
   - Expect canonical `defmodule ... do` / `def ... do` / `end` structure with a blank line between functions.
2. **Private function with defaults and guard**
   - Parse a `defp` with a default arg and `when` guard.
   - Expect exact header rendering.
3. **Call wrapping**
   - Parse a function body with a long call.
   - At wide width, expect a flat single-line call.
   - At narrow width, expect one-arg-per-line style.
4. **Pipe chain wrapping**
   - Parse a pipe chain that exceeds width.
   - Expect the chain to break before each `|>`.
5. **Block body formatting**
   - Parse a function body with multiple expressions.
   - Expect stable line separation and indentation.
6. **Live-path regression**
   - Re-run the existing comment/idempotency and CLI parity regression tests to prove the unchanged token formatter still behaves.
7. **Touched-surface regression**
   - Re-run `formatter::algebra` after adding `SoftLine` so the algebra change is covered explicitly.

## Verification commands
- `cargo test formatter::to_doc -- --nocapture`
- `cargo test formatter::algebra -- --nocapture`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture`

## Expected files
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

## Critic note
Unless the builder deliberately rewires `format_source` to use the AST converter — which is out of scope for this slice — there is no honest manual smoke path into the new code. The critic should reject any claim that a CLI run exercised `src/formatter/to_doc.rs`.
