# Plan

## Active slice
Extend `src/formatter/to_doc.rs` over collection/data literals and `defstruct` module forms, while keeping the AST formatter completely off the live `tonic fmt` path.

## Why this slice now
The AST formatter already proves the algebra works for modules/functions/calls/pipes. The next coherent gap is data-shape rendering: today the AST path still rejects tuples, lists, keywords, maps, structs, updates, and any module that contains `defstruct`. That is a narrow enough family to test well, and it unlocks realistic formatter fixtures without mixing in CLI routing, comments, or control-flow work.

## Builder checklist
- [x] Re-read `src/formatter/to_doc.rs`, `src/formatter/algebra.rs`, `src/formatter/mod.rs`, `src/parser/ast/mod.rs`, and the relevant collection fixtures in `src/parser/tests.rs` before editing.
- [x] Keep `src/formatter/mod.rs::format_source` wired to `engine::format_source_inner` in this slice.
- [x] Teach `module_to_doc` to render `ModuleForm::Defstruct`.
- [x] Preserve stable blank-line separation between rendered module forms and rendered functions.
- [x] Add AST doc rendering for tuple literals.
- [x] Add AST doc rendering for list literals.
- [x] Add AST doc rendering for keyword literals.
- [x] Add AST doc rendering for map literals with mixed key styles.
- [x] Add AST doc rendering for struct literals.
- [x] Add AST doc rendering for map-update and struct-update expressions.
- [x] Use algebra groups/nesting so short collections stay flat and narrow widths break to one-entry-per-line style.
- [x] Add focused unit tests that parse source, convert to docs, render with chosen widths, and assert exact strings for the new supported family.
- [x] Keep module attributes, alias/import/require/use/protocol forms, comment reinsertion, CLI/config wiring, and broader control-flow families out of scope.
- [x] Save command output under `logs/` and do not stage unrelated dirty files.

## Test plan
1. **Defstruct module shell**
   - Parse a module with `defstruct` and a function.
   - Expect `defstruct` to render before functions with a single blank line separator.
2. **Tuple/list literals**
   - Parse flat tuple/list expressions.
   - At wide width, expect a single line.
   - At narrow width, expect one item per line with closing delimiter aligned.
3. **Keyword/map literals**
   - Parse keyword entries and mixed-key map entries.
   - Verify exact rendering for both flat and broken widths.
4. **Struct literals and updates**
   - Parse `%User{name: "A"}` and `%User{user | age: 43}`.
   - Verify exact rendering, including broken layout when width is tight.
5. **Nested collection body**
   - Parse a function body that nests a struct/map/list combination.
   - Expect stable indentation and delimiter placement.
6. **Live-path regression**
   - Re-run the existing comment/idempotency and CLI parity regression tests to prove the unchanged token formatter still behaves.
7. **Touched-surface regression**
   - Re-run `formatter::algebra` after any collection-layout helper changes.

## Verification commands
- `cargo test formatter::to_doc -- --nocapture > logs/formatter_to_doc_collections.log 2>&1`
- `cargo test formatter::algebra -- --nocapture > logs/formatter_algebra.log 2>&1`
- `cargo test format_source_is_idempotent_with_comments -- --nocapture > logs/formatter_regression.log 2>&1`
- `cargo test --test fmt_parity_smoke fmt_preserves_comments_and_is_idempotent -- --nocapture > logs/formatter_parity.log 2>&1`

## Expected files
- `src/formatter/to_doc.rs`
- `context.md`
- `plan.md`
- `progress.md`
- `logs/formatter_to_doc_collections.log`
- `logs/formatter_algebra.log`
- `logs/formatter_regression.log`
- `logs/formatter_parity.log`

## Critic note
Unless the builder deliberately rewires `format_source` to use the AST converter — which is out of scope for this slice — there is no honest manual smoke path into the new code. The critic should reject any claim that a CLI run exercised `src/formatter/to_doc.rs`.
