## Deferred optimization ideas

- Split `stubs_try.rs` (562 lines) further — `emit_try_ops` is 270 lines of match arms that could potentially share code with closure body emission
- Split `stubs_for.rs` (778 lines) — the static for evaluation engine could be its own module
- `resolver_diag.rs` (509 lines) — extract test module to separate file to get under 500
- `bin/benchsuite/model.rs` (534 lines) — split SLO/contract types from workload types
- `interop/string_mod.rs` (526 lines) — extract validation helpers
- The 3 partial parity items (bitstring runtime, universal hints, use/require stubs) are each significant efforts
- P1 stdlib gap "Runtime text is still binary-shaped" is a deep runtime design decision
- Consider adding `Enum.map/2`, `Enum.filter/2`, `Enum.reduce/3` to close P2 collection surface gap
- Many large files (runtime.rs 1669, ir.rs 2231, lexer.rs 2083, main.rs 2363) need architectural splits
