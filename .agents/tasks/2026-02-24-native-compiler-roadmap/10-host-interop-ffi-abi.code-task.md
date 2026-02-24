---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Define and Implement Host Interop ABI for Native Backend

## Description
Provide a stable host function ABI usable from native-compiled artifacts, preserving current `host_call` semantics and deterministic error behavior.

## Background
Interpreter mode calls host registry directly in Rust. Native backend requires a call boundary that supports dynamic values, arity checks, and robust error translation.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/interop.rs`
- `src/runtime.rs`
- `tests/check_host_call_typing.rs`
- `tests/run_protocol_dispatch_smoke.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define host-call ABI signatures for native runtime values and error returns.
2. Implement adapter layer between compiled artifacts and existing host registry.
3. Preserve atom-key validation and arity/type diagnostics.
4. Support protocol-dispatch helper interactions needed by native mode.
5. Add compatibility policy/versioning for future host ABI evolution.

## Dependencies
- Runtime ABI (task 03).
- Native runtime helper library (task 04).
- LLVM backend call support (task 06+).

## Implementation Approach
1. Add interop shim module exposing backend-safe entrypoints.
2. Implement conversion and error-bridge logic between `TValue` and host APIs.
3. Add integration tests for successful and failing host calls in native mode.

## Acceptance Criteria

1. **Host Calls Work in Native Artifacts**
   - Given compiled programs invoking `host_call`
   - When run with registered host functions
   - Then returned values and side effects match interpreter behavior.

2. **Interop Failures Are Deterministic**
   - Given unknown host keys, bad arity, or invalid argument types
   - When running native artifacts
   - Then errors are deterministic and aligned with existing contracts.

3. **Protocol Dispatch Path Remains Compatible**
   - Given protocol dispatch fixtures
   - When executed in native mode
   - Then dispatch behavior and output are unchanged.

4. **Unit/Integration Tests Cover Host ABI Boundary**
   - Given new interop ABI adapter code
   - When running `cargo test`
   - Then tests validate conversion logic, error mapping, and compatibility behavior.

## Metadata
- **Complexity**: Medium
- **Labels**: Interop, ABI, Host-Functions, LLVM
- **Required Skills**: ABI boundaries, runtime value conversion, robust error handling
