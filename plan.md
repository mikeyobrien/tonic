# Plan

## Active slice
Plan and then implement interpolation-aware text blocks: `~t""" ... #{...} ... """` using the existing interpolated-string pipeline.

## Why this slice now
The plain text-block slice is already landed at `13962bc`, but the original task is not complete until `#{}` works inside text blocks. This is the next honest slice because it finishes the feature without reopening unrelated formatter or runtime work.

## Planner checklist
- [x] Re-read `context.md`, `plan.md`, `progress.md`, `.agents/tasks/2026-03-27-text-block-ergonomics/dedented-text-block-sigil.code-task.md`, `src/lexer/string_scan.rs`, `src/lexer/tests.rs`, `src/parser/literal.rs`, `src/parser/expr.rs`, `tests/check_dump_ast_string_interpolation.rs`, `tests/run_primitives_smoke.rs`, and `TONIC_REFERENCE.md` before changing the plan.
- [x] Decide the narrowest lexer strategy that preserves text-block dedent rules while emitting the existing interpolation token sequence.
- [x] Define exact dedent behavior when `#{...}` appears on mixed-indentation lines or spans multiple source lines.
- [x] Replace the current "reject text-block interpolation" expectation with positive coverage and sharp malformed-input diagnostics.
- [x] Keep the slice limited to text-block interpolation, tests, and docs. Do not widen into formatter or unrelated string work.
- [x] Hand builder a concrete verification list and name the authoritative review target as current HEAD once planned.

## Expected builder work
- [x] Extend `src/lexer/string_scan.rs` so `~t"""...#{...}..."""` can produce the normal interpolated-string token flow after text-block normalization.
- [x] Preserve the already-landed plain-text-block behavior.
- [x] Keep raw heredoc behavior unchanged.
- [x] Update lexer tests for positive interpolation coverage plus malformed/unterminated diagnostics.
- [x] Add/extend AST dump regression coverage in `tests/check_dump_ast_string_interpolation.rs`.
- [x] Add runtime coverage for interpolated text blocks in `tests/run_primitives_smoke.rs` or a nearby focused test.
- [x] Update `TONIC_REFERENCE.md` to remove the current "not supported yet" note and document interpolation semantics for text blocks.
- [x] Save verification output under `logs/`.

## Verification plan
1. **Lexer / tokenization**
   - interpolated text block emits the existing `StringStart` / `StringPart` / `InterpolationStart` / `InterpolationEnd` / `StringEnd` flow
   - dedent still ignores blank lines and preserves relative indentation around interpolation boundaries
2. **Diagnostics**
   - malformed `#{...}` inside `~t"""` reports a precise error
   - unterminated `~t"""` still reports the correct text-block error span
3. **AST shape**
   - `tonic check --dump-ast` shows an `interpolatedstring` shape for the new syntax rather than a new special-case node
4. **Runtime behavior**
   - `tonic run` proves dedent + interpolation both work on the live path
5. **Regression**
   - existing plain text-block and raw heredoc behavior remain green

## Suggested verification commands
- `mkdir -p logs && cargo test text_block -- --nocapture > logs/text_block_lexer.log 2>&1`
- `mkdir -p logs && cargo test --test check_dump_ast_string_interpolation -- --nocapture > logs/text_block_ast_interpolation.log 2>&1`
- `mkdir -p logs && cargo test --test run_primitives_smoke run_executes_text_block_literals_with_trimmed_dedent_contract -- --exact > logs/text_block_runtime_plain.log 2>&1`
- `mkdir -p logs && cargo test --test run_primitives_smoke <new_interpolated_text_block_test_name> -- --exact > logs/text_block_runtime_interpolation.log 2>&1`
- `mkdir -p logs && cargo test heredoc -- --nocapture > logs/text_block_heredoc_regression.log 2>&1`

## Critic expectation for the next review
Critic should review the post-slice HEAD, not `13962bc` alone, and independently smoke an interpolated fixture with:
- `cargo run --bin tonic -- run <fixture-containing-~t-and-#{...}>`

That smoke must hit the changed lexer path directly.

## Explicitly out of scope
- formatter support
- self-hosted lexer parity expansion unless tiny
- changing raw heredoc semantics
- broader string system redesign
