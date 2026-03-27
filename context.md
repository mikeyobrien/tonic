# Context

## Objective
Implement the dedented text-block task incrementally.

### Active slice
Add a non-interpolated `~t""" ... """` text-block sigil that:
- trims one optional framing newline after the opener
- trims one optional framing newline before the closer
- removes the minimum common indentation across non-blank content lines
- preserves relative indentation after dedent
- leaves raw `"""..."""` heredocs unchanged

Interpolated text blocks (`~t"""...#{...}..."""`) are explicitly **next slice**, not this one.

## Source task
- Primary task file: `.agents/tasks/2026-03-27-text-block-ergonomics/dedented-text-block-sigil.code-task.md`
- The prompt said `.agents/planning`, but the actual implementation brief lives under `.agents/tasks/...`.

## Baseline repo state
- Baseline commit before this planning pass: `26e491c`
- The worktree is already dirty in many unrelated files; this slice must avoid touching or staging unrelated paths.

## Existing implementation facts
- `src/lexer/string_scan.rs`
  - `scan_string_literal` handles `"..."` and raw heredocs `"""..."""`
  - interpolation uses the existing token flow: `StringStart` / `StringPart` / `InterpolationStart` / `InterpolationEnd` / `StringEnd`
  - `scan_sigil` currently supports `~s`, `~r`, and `~w`
- `src/lexer/mod.rs`
  - lexer state only distinguishes `Normal` vs `String { is_heredoc, brace_depth }`
  - `~` dispatch already routes into `string_scan::scan_sigil`
- `src/parser/expr.rs` and `src/parser/literal.rs`
  - plain `TokenKind::String` lowers straight to `Expr::String`
  - interpolated string tokens lower to `Expr::InterpolatedString`
- `src/ir_lower_expr.rs`
  - `Expr::String` already lowers to `IrOp::ConstString`
  - plain text-block literals can therefore reuse the existing runtime path with no new runtime node
- Existing regression surfaces
  - lexer unit tests: `src/lexer/tests.rs`, `src/lexer/tests_extended.rs`
  - AST dump regression: `tests/check_dump_ast_expressions.rs`
  - runtime regression: `tests/run_primitives_smoke.rs`
  - user docs: `TONIC_REFERENCE.md`

## Recommended implementation shape for this slice
1. Extend `scan_sigil` to recognize only the dedicated triple-quoted text-block form: `~t"""..."""`.
2. Keep the output surface narrow by normalizing the raw text-block contents in the lexer and then emitting an ordinary `TokenKind::String`.
3. Centralize trim/dedent logic in one helper inside `src/lexer/string_scan.rs` so edge cases are unit-testable without parser/runtime noise.
4. Keep raw heredoc scanning untouched.
5. Keep parser, AST, resolver, typing, and lowering changes minimal or zero unless span/diagnostic plumbing truly requires otherwise.

## Edge cases this slice should define and test
- fully blank block → empty string after framing trim/dedent
- one content line
- mixed blank and indented lines
- a less-indented line setting the common indent floor
- unterminated `~t"""` block
- invalid `~t` sigil spellings that are not the supported triple-quoted form

## Intentionally out of scope for this slice
- interpolation inside `~t` blocks
- new AST/runtime string node types
- formatter support
- self-hosted lexer parity expansion unless it stays tiny and does not widen the slice
