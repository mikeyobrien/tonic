---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Add Native Support for Anonymous Functions, Captures, and Invocation

## Description
Implement LLVM/native support for closure creation, lexical capture, capture shorthand, and invocation semantics.

## Background
Closures are central to idiomatic scripting and pipelines. Native backend parity requires robust captured-environment handling, not just direct function calls.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/runtime.rs`
- `tests/run_anon_fn_capture_smoke.rs`
- `tests/check_dump_ir_result_case.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define closure object representation in native ABI/runtime.
2. Lower anonymous function creation to closure allocation with captured bindings.
3. Support capture shorthand semantics and function-value invocation (`fun.(x)`).
4. Preserve lexical scoping semantics and deterministic runtime failures for arity/type mismatches.
5. Ensure closures can cross function boundaries and remain valid.

## Dependencies
- Runtime ABI/memory model (task 03).
- Native helper library (task 04).
- LLVM call/control support (task 06).

## Implementation Approach
1. Add closure runtime structure and environment capture APIs.
2. Implement MIR/LLVM lowering for closure create/invoke ops.
3. Add parity/differential tests for capture and invocation scenarios.

## Acceptance Criteria

1. **Lexical Capture Works in Native Mode**
   - Given fixtures with outer-scope captures
   - When compiled/executed
   - Then captured values and outputs match interpreter mode.

2. **Capture Shorthand and Invoke Semantics Match**
   - Given shorthand and explicit anon-fn fixtures
   - When run in native mode
   - Then invocation results and arity behavior remain correct.

3. **Closure Lifetime Is Safe**
   - Given closures that outlive their defining stack scope
   - When executed repeatedly
   - Then no invalid memory behavior occurs and semantics remain stable.

4. **Unit/Integration Tests Cover Closure Paths**
   - Given closure lowering and runtime APIs
   - When running `cargo test`
   - Then tests cover creation, invocation, capture, and failure behavior.

## Metadata
- **Complexity**: High
- **Labels**: LLVM, Closures, Capture, Runtime
- **Required Skills**: Closure representation, memory ownership, codegen for function values
