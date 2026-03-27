# Context

## Objective
Implement `code-task.formatter.md` incrementally. The current `tonic fmt` is token-driven, strips comments, and has no width-aware wrapping. The first slice should improve correctness without trying to land the full AST/algebra formatter in one step.

## Relevant code
- `src/formatter/mod.rs` — `format_source` / file traversal / formatter tests
- `src/formatter/engine.rs` — current token-driven logical-line builder + indentation engine
- `src/lexer/mod.rs` — scanner currently skips `# ...` comments entirely
- `src/lexer/types.rs` — token/span types; likely home for comment sidecar type
- `src/lexer/tests.rs` — current lexer behavior asserts comments are skipped
- `tests/fmt_parity_smoke.rs` — CLI-level formatter smoke tests
- `src/parser/*` — future AST-driven formatter work, but not required for the first slice

## Current behavior
- `scan_tokens` drops comments before the formatter sees them.
- `format_source` delegates to the token formatter and returns normalized source unchanged on lex failure.
- The formatter already preserves one blank line and indentation for `do`/`end`/branch constructs.

## Constraints and planning call
- The task is much larger than a single safe slice; start with comment preservation on top of the existing formatter.
- Do not start the Wadler-Lindig / AST printer in the same slice unless comment preservation lands cleanly first.
- Because the repo is already dirty outside the formatter surface, only touch and later stage formatter-task files.

## First slice target
Add a comment sidecar in the lexer plus formatter support that preserves:
- full-line comments
- trailing comments on code lines
- a single blank line between logical regions
- idempotency for comment-bearing files

## Likely design
1. Extend the lexer with a public comment-aware scan entrypoint (keep `scan_tokens` behavior-compatible for non-formatter callers if possible).
2. Record comment metadata needed by the formatter: span/line/column/text and whether blank lines precede the comment.
3. Teach `src/formatter/engine.rs` to merge comment sidecar entries back into logical lines using source positions.
4. Add unit tests first for lexer comment capture and formatter round-trip behavior, then implement.

## Verification surfaces for slice 1
- Focused Rust tests for lexer + formatter modules
- CLI smoke test covering `tonic fmt` on a comment-bearing fixture
- Idempotency rerun on the formatted comment fixture
