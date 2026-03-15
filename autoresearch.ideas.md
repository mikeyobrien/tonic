## Deferred optimization ideas

- Split `resolver.rs` (1086 lines) — extract ModuleGraph building (~452 lines) to resolver_graph.rs
- Split `manifest.rs` (1239 lines) — extract stdlib injection and project source loading
- Split `interop.rs` (1580 lines) — extract host registry and individual module registrations
- Split `runtime.rs` (1673 lines) — extract evaluate_ops match arms into subsystems
- Split `ir.rs` (2243 lines) — extract lower_expr match arms, pattern lowering
- Split `main.rs` (2363 lines) — extract command handlers (run, check, compile, test, fmt, etc.)
- Split `lexer.rs` (2083 lines) — extract token scanning for strings/numbers/comments
- Split `c_backend/stubs.rs` (3484 lines) — the C runtime string literal is one massive function
- `stubs_try.rs` (562 lines) — `emit_try_ops` is 270 lines, hard to split without architecture change
- `stubs_for.rs` (778 lines) — static for evaluation engine could be its own module
- `runtime_patterns.rs` (574 lines) — `emit_pattern_case` is 232 lines, one big match
- The 2 remaining partial parity items (bitstring runtime, universal hints) need significant work
- Consider adding `Enum.map/2`, `Enum.filter/2`, `Enum.reduce/3` to close P2 collection surface gap
- Case/cond branch bodies still limited to single expressions (need newline-aware parsing to fix)
