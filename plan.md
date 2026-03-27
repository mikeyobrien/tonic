# Plan

## Active slice
Extend `src/formatter/to_doc.rs` over `case` expressions and pattern rendering so the AST formatter can render direct `case` plus parser-lowered `if`/`unless`/`cond`/`with`, while keeping the AST formatter completely off the live `tonic fmt` path.

## Why this slice now
The AST formatter already covers functions, calls, pipes, and collection/data literals, but it still cannot render branching. The parser lowers several surface control forms into `Expr::Case`, so one library-only slice on case/pattern formatting unlocks a large syntax family without mixing in runtime wiring, comments, or unrelated module-form work.

## Builder checklist
- [x] Re-read `src/formatter/to_doc.rs`, `src/formatter/algebra.rs`, `src/formatter/mod.rs`, `src/parser/ast/mod.rs`, `src/parser/ast/expr_def.rs`, `tests/check_dump_ast_control_forms.rs`, and `tests/check_dump_ast_case_patterns.rs` before editing.
- [x] Keep `src/formatter/mod.rs::format_source` wired to `engine::format_source_inner` in this slice.
- [x] Teach `expr_to_doc` to render `Expr::Case`.
- [x] Add `pattern_to_doc` helpers for wildcard, bind, pin, literal, tuple, list, map, and struct patterns needed by current control-form lowering/tests.
- [x] Render branch guards as `pattern when guard -> body` when present.
- [x] Preserve stable indentation for branch bodies, including nested block bodies and nested `case` expressions.
- [x] Add focused unit tests that parse source, convert to docs, render with chosen widths, and assert exact strings for direct `case` plus lowered `if`/`unless`/`cond`/`with` examples.
- [x] Keep module attributes/forms, comment reinsertion, `try`/`raise`, `for`, anonymous functions, interpolation, bitstrings, and CLI/config/runtime wiring out of scope.
- [x] Save command output under `logs/` and do not stage unrelated dirty files.
- [x] Commit the slice before `review.ready` and cite the commit hash in the handoff.

## Test plan
1. **Direct case with patterns and guard**
   - Parse a `case` using tuple/list/map/struct/literal/pin/wildcard patterns.
   - Assert exact rendering, including `when` guards.
2. **Lowered if/unless**
   - Parse canonical `if` / `unless` source.
   - Expect AST formatting to round-trip through the lowered `case` representation into stable multi-line output.
3. **Lowered cond/with**
   - Parse `cond` and `with` examples from existing parser contracts.
   - Assert the AST formatter renders the nested case structure deterministically.
4. **Nested branch bodies**
   - Parse a `case` branch whose body is a block or nested `case`.
   - Expect indentation and `end` alignment to stay exact.
5. **Live-path regression**
   - Re-run the existing comment/idempotency and CLI parity regression tests to prove the unchanged token formatter still behaves.
6. **Touched-surface regression**
   - Re-run `formatter::algebra` after any helper/layout changes.

## Verification commands
- `cargo test formatter::to_doc -- --nocapture > logs/formatter_to_doc_control.log 2>&1`
- `cargo test formatter::algebra -- --nocapture > logs/formatter_algebra.log 2>&1`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture > logs/formatter_regression.log 2>&1`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture > logs/formatter_parity.log 2>&1`

## Expected files
- `src/formatter/to_doc.rs`
- `context.md`
- `plan.md`
- `progress.md`
- `logs/formatter_to_doc_control.log`
- `logs/formatter_algebra.log`
- `logs/formatter_regression.log`
- `logs/formatter_parity.log`

## Critic note
Unless the builder deliberately rewires `format_source` to use the AST converter — which is out of scope for this slice — there is no honest manual smoke path into `src/formatter/to_doc.rs`. The critic should reject any claim that a CLI run exercised the new case/pattern formatter code.
