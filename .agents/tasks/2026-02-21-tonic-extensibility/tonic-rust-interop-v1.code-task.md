# Task: Add Rust Host Interop (v1) via `host_call` and Static Extension Registry

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Description
Implement a first-class Rust interop layer so Tonic programs can call statically linked Rust host functions. The v1 design should prioritize deterministic behavior, low startup overhead, and strong diagnostics over dynamic plugin complexity.

## Background
Tonic currently executes lowered IR with a fixed builtin set in `src/runtime.rs`. There is no supported way to extend runtime behavior from Rust code other than editing core builtins. A host interop boundary enables:
- high-performance native functionality
- controlled access to OS/network APIs
- extensibility without changing language core each time

Given v0/v1 constraints, interop should avoid runtime dynamic loading and use a static registry model.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Runtime evaluator: `src/runtime.rs`
- IR/lowering flow: `src/ir.rs`
- Resolver behavior and builtin handling: `src/resolver.rs`
- Type inference handling for builtins: `src/typing.rs`
- CLI wiring and compile pipeline: `src/main.rs`
- Reliability hardening tasks: `.agents/tasks/2026-02-21-tonic-reliability/`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Introduce a host interop API with a stable Rust signature for host functions (example shape):
   - `type HostFn = fn(&[RuntimeValue]) -> Result<RuntimeValue, HostError>`
2. Add a static extension registry mapping atom keys (e.g., `:fs_read`) to host functions.
3. Add a language/runtime entrypoint for interop, e.g. builtin `host_call/2+`:
   - first arg must be atom host key
   - remaining args forwarded to registered function
4. Implement strict argument validation and deterministic errors for:
   - non-atom function keys
   - unknown host function key
   - arity/type mismatch returned from host function
5. Keep interop deterministic and single-process safe (no dynamic plugin loading in v1).
6. Preserve startup/memory constraints by avoiding heavyweight reflection or runtime discovery.
7. Integrate with resolver and typing so interop calls pass static pipeline checks:
   - resolver recognizes `host_call`
   - typing enforces first arg type enough for clear diagnostics (or documented dynamic fallback)
8. Ensure runtime errors from host functions are surfaced through existing `RuntimeError` contract.
9. Add extension registration wiring at startup (single registry build path).
10. Add automated tests for success and failure behavior, including end-to-end fixtures.

## Dependencies
- `src/runtime.rs` builtin dispatch and `RuntimeValue`
- `src/resolver.rs` builtin allowlist logic
- `src/typing.rs` builtin type inference rules
- `src/main.rs`/bootstrapping for registry initialization
- Integration tests in `tests/`

## Implementation Approach
1. Create a focused interop module (`src/interop.rs`) containing:
   - host function type
   - host error type
   - registry structure and lookup
2. Add `host_call` builtin handling in runtime:
   - decode first arg atom key
   - lookup and invoke host function
   - map `HostError` -> `RuntimeError`
3. Update resolver builtin target list to include `host_call`.
4. Update typing builtin inference for `host_call` with pragmatic v1 typing policy.
5. Add a small built-in sample host function set (`:identity`, `:sum_ints` etc.) for testability.
6. Add integration tests covering:
   - happy path host calls
   - unknown key
   - wrong key type
   - host function error mapping
7. Document v1 interop limits and non-goals (no dynamic library loading yet).

## Acceptance Criteria

1. **Interop Happy Path**
   - Given a registered host function key and valid arguments
   - When `host_call(:key, ...)` is executed
   - Then runtime returns the host function result deterministically

2. **Unknown Host Key Diagnostic**
   - Given an unregistered host function key
   - When `host_call(:missing, ...)` executes
   - Then runtime fails with deterministic unknown-host-function diagnostics

3. **Invalid Key Type Handling**
   - Given a non-atom first argument to `host_call`
   - When evaluation runs
   - Then runtime fails with deterministic argument-type diagnostics

4. **Host Error Mapping**
   - Given a host function that returns an error
   - When invoked via `host_call`
   - Then error is surfaced through Tonic runtime error contract with actionable message

5. **Compile Pipeline Compatibility**
   - Given source using `host_call`
   - When `tonic check` and `tonic run` execute
   - Then resolver/type/lowering/runtime stages succeed or fail deterministically as appropriate

6. **Regression Safety**
   - Given existing runtime and CLI fixtures
   - When full tests run
   - Then existing behavior remains unchanged

## Metadata
- **Complexity**: Medium
- **Labels**: Interop, Rust, Runtime, Builtins, Extensibility
- **Required Skills**: Rust API design, runtime error modeling, compiler pipeline integration, integration testing