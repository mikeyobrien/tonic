## Deferred optimization ideas

- Split `ir.rs` (2243 lines) — extract lower_expr match arms, pattern lowering
- Split `main.rs` (2363 lines) — extract command handlers (run, check, compile, test, fmt, etc.)
- Split `lexer.rs` (2083 lines) — extract token scanning for strings/numbers/comments
- Split `c_backend/stubs.rs` (3484 lines) — needs finer-grained split than 5 files; each helper must stay under 500 lines. Consider ~8-10 files targeting ~350-400 lines each, splitting the raw string literals at logical C function boundaries
- Split `runtime.rs` (1673 lines) — extract evaluate_ops match arms into subsystems
- Split `llvm_backend/codegen.rs` (1716 lines) — extract codegen match arms
- The 2 remaining partial parity items (bitstring runtime, universal hints) need significant work
- Consider adding `Enum.map/2`, `Enum.filter/2`, `Enum.reduce/3` to close P2 collection surface gap
- Case/cond branch bodies still limited to single expressions (need newline-aware parsing to fix)
- Rust allows inherent impls in child modules — useful for splitting large impl blocks
