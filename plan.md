# Plan

## Active slice
Comment-preserving token formatter foundation.

## Why this slice first
It removes the most destructive current behavior (`tonic fmt` deleting comments) while staying small enough to verify. The full AST/algebra formatter remains the next major slice.

## Builder checklist
- [x] Add/adjust lexer-side comment model in `src/lexer/types.rs` and `src/lexer/mod.rs`
- [x] Keep existing token scanning callers working, or introduce a narrow comment-aware formatter-only entrypoint
- [x] Update lexer tests to assert comment capture instead of silent discard
- [x] Update formatter engine to reinsert full-line and trailing comments deterministically
- [x] Add formatter unit tests for comment preservation and idempotency
- [x] Add/extend CLI smoke coverage in `tests/fmt_parity_smoke.rs`
- [x] Run focused verification commands and save outputs under `logs/`
- [x] Commit only the formatter slice files once verification passes

## Test plan
1. **Lexer capture test**
   - Input: `1 # trailing\n# heading\n2\n`
   - Expect: numeric tokens still parse; comments are captured with stable positions/text.
2. **Formatter full-line comment preservation**
   - Input: unformatted nested block with leading comment
   - Expect: comment remains, indentation is normalized, trailing newline contract preserved.
3. **Formatter trailing comment preservation**
   - Input: `def run() do\n1 # note\nend\n`
   - Expect: trailing comment remains attached to the `1` line after formatting.
4. **Formatter idempotency with comments**
   - Format twice; second pass must equal first.
5. **CLI smoke**
   - Run `tonic fmt` on a temp fixture containing comments; file content should retain comments and second pass should be a no-op.

## Verification commands
- `cargo test formatter:: lexer::tests::scan_tokens_*comment* -- --nocapture`
- `cargo test fmt_parity_smoke -- --nocapture`
- If the focused commands are noisy or insufficient, run the exact affected tests by name and capture output to `logs/`.

## Explicit deferrals after slice 1
- Wadler-Lindig algebra engine (`src/formatter/algebra.rs`)
- AST-to-doc conversion (`src/formatter/to_doc.rs`)
- width-aware wrapping / `--line-length`
- `.tonic_formatter` config

These should become the next slices once comment preservation is green.
