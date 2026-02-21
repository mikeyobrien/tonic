# Progress

## Status
Completed

## Tasks
- [x] Explore codebase (lexer, parser, IR, runtime, typing, existing tests)
- [x] Lexer: add Minus, Star, Slash, EqEq, BangEq, Lt, LtEq, Gt, GtEq tokens
- [x] Parser: add BinaryOp variants + precedence table (* > + > comparisons)
- [x] IR: add SubInt, MulInt, DivInt, CmpInt ops; CmpKind enum
- [x] Runtime: execute new ops; division-by-zero guard; CmpInt → Bool
- [x] Typing: distinguish arithmetic (→Int) from comparison (→Bool)
- [x] Tests: AST dump snapshots for precedence and comparison
- [x] Tests: run smoke tests for sub/mul/div/precedence/comparisons/div-by-zero/type-error
- [x] All tests pass (cargo test: 0 failed)
- [x] Commit: `feat(parity): add arithmetic and comparison operators` (0814a26)

## Files Changed
- `src/lexer.rs` — 9 new token kinds; '-' now emits Minus when not followed by '>'
- `src/parser.rs` — 9 new BinaryOp variants; updated `current_binary_operator` precedence table
- `src/ir.rs` — SubInt, MulInt, DivInt, CmpInt{kind:CmpKind} ops; CmpKind enum; updated lowering
- `src/runtime.rs` — 4 new op handlers; div-by-zero guard; CmpInt produces RuntimeValue::Bool
- `src/typing.rs` — Binary inference returns Bool for comparisons, Int for arithmetic
- `tests/check_dump_ast_expressions.rs` — 2 new golden snapshot tests
- `tests/run_arithmetic_smoke.rs` — 8 new smoke tests

## Notes
- `CmpInt` field named `kind` (not `op`) to avoid conflict with serde `tag = "op"` on IrOp enum
- `pop_int` error message updated to generic "int operator" since now shared by all arithmetic ops
- Type diagnostics verified: `true + 1` produces `[E2001] type mismatch: expected int, found bool at offset 37`
