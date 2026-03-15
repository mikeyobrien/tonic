## Deferred optimization ideas

- Split `stubs_try.rs` (562 lines) further — `emit_try_ops` is 270 lines of match arms that could potentially share code with closure body emission
- Split `stubs_for.rs` (778 lines) — the static for evaluation engine could be its own module
- `bin/benchsuite/model.rs` (534 lines) — extract test module or split SLO/contract types from workload types
- `interop/string_mod.rs` (526 lines) — extract test module (non-test code is only 306 lines)
- `parser/expr.rs` (503 lines) — 3 lines over, could move operator table to separate file
- The 3 partial parity items (bitstring runtime, universal hints, use/require stubs) are each significant efforts
- Consider adding `Enum.map/2`, `Enum.filter/2`, `Enum.reduce/3` to close P2 collection surface gap
- Many large files (runtime.rs 1669, ir.rs 2231, lexer.rs 2083, main.rs 2363) need architectural splits
- Case/cond branch bodies still limited to single expressions (need newline-aware parsing to fix)
- Add regression tests for Block expression in all supported contexts (def, if, for, try, with)
