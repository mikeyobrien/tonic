# Plan

## Active slice
Implement plain dedented text blocks: `~t""" ... """` → existing `TokenKind::String` / `Expr::String` / runtime string flow.

## Why this slice now
The normalization rules are the core ergonomics win, and the non-interpolated path is the smallest honest end-to-end slice. It exercises the live compiler/runtime path without taking on the harder interpolation-plus-dedent state machine in the same commit.

## Builder checklist
- [ ] Re-read `context.md`, `plan.md`, `progress.md`, `src/lexer/string_scan.rs`, `src/lexer/mod.rs`, `src/lexer/tests.rs`, `src/lexer/tests_extended.rs`, `tests/check_dump_ast_expressions.rs`, `tests/run_primitives_smoke.rs`, and `TONIC_REFERENCE.md` before editing.
- [ ] Add a narrow normalization helper in `src/lexer/string_scan.rs` for text-block trim/dedent behavior.
- [ ] Extend sigil scanning to recognize `~t"""..."""` only in this slice.
- [ ] Emit existing `TokenKind::String` with normalized content for non-interpolated text blocks.
- [ ] Keep raw `"""..."""` heredoc behavior unchanged.
- [ ] Add focused lexer tests for trim/dedent edge cases and invalid/unterminated `~t` diagnostics.
- [ ] Add an AST dump regression proving the new syntax still lowers to the existing plain string AST shape.
- [ ] Add a runtime regression proving `cargo run --bin tonic -- run` produces the dedented result on the live path.
- [ ] Update `TONIC_REFERENCE.md` to explain raw heredoc vs plain dedented text block, without claiming interpolation support yet.
- [ ] Save verification output under `logs/`.
- [ ] Commit the slice before `review.ready` and record the commit hash in `progress.md`.

## Test plan
1. **Lexer normalization**
   - `~t"""\n  hello\n  world\n"""` → `STRING(hello\nworld)`
   - blank lines do not increase dedent
   - a less-indented non-blank line sets the minimum indent floor
   - fully blank block normalizes to empty string
2. **Diagnostics**
   - unterminated `~t"""` reports the same sharp span style as other string errors
   - unsupported `~t` spellings fail cleanly instead of being mis-tokenized as some other sigil
3. **AST shape**
   - `tonic check --dump-ast` shows the new literal as the existing `{"kind":"string",...}` shape
4. **Runtime behavior**
   - `tonic run` prints the dedented string exactly, proving the live path works
5. **Regression**
   - existing raw heredoc behavior still preserves newlines exactly

## Verification commands
- `mkdir -p logs && cargo test text_block -- --nocapture > logs/text_block_lexer.log 2>&1`
- `mkdir -p logs && cargo test heredoc -- --nocapture > logs/text_block_heredoc_regression.log 2>&1`
- `mkdir -p logs && cargo test --test check_dump_ast_expressions check_dump_ast_matches_text_block_literal_contract -- --exact > logs/text_block_ast.log 2>&1`
- `mkdir -p logs && cargo test --test run_primitives_smoke run_executes_text_block_literals_with_trimmed_dedent_contract -- --exact > logs/text_block_runtime.log 2>&1`

## Critic manual smoke expectation
This slice has a real manual surface. Critic should independently create or use a fixture containing `~t"""..."""` and run:
- `cargo run --bin tonic -- run <fixture>`

That smoke must hit the changed lexer path directly; CLI evidence is honest proof for this slice.

## Expected files
- `src/lexer/string_scan.rs`
- `src/lexer/mod.rs` (only if state/plumbing changes are truly needed)
- `src/lexer/tests.rs`
- `src/lexer/tests_extended.rs`
- `tests/check_dump_ast_expressions.rs`
- `tests/run_primitives_smoke.rs`
- `TONIC_REFERENCE.md`
- `context.md`
- `plan.md`
- `progress.md`
- `logs/text_block_lexer.log`
- `logs/text_block_heredoc_regression.log`
- `logs/text_block_ast.log`
- `logs/text_block_runtime.log`

## Explicitly deferred to next slice
- `~t"""...#{...}..."""` interpolation support
- interpolation-aware dedent rules
- any self-hosted lexer parity expansion for the new syntax
