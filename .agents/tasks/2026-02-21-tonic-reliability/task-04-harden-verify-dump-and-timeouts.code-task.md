# Task: Harden Verify Gates, Dump Modes, and Timeout Controls

## Description
Stabilize verification and introspection command behavior by hardening verify threshold enforcement, dump mode contracts, and execution timeout handling for external process interop (if present).

## Background
Reliable automation needs deterministic introspection (`--dump-*`) and deterministic verification gate failures. Timeout controls are critical where external command paths exist, otherwise a single hung command can stall CI and developer workflows.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Verify runner and command handling: `src/main.rs`, `src/acceptance.rs`
- Benchmark gates and evidence behavior tests: `tests/verify_*`, `tests/verify_benchmark_gate_thresholds.rs`
- Dump mode behavior in `check`: `src/main.rs`, dump-related tests in `tests/check_dump_*`
- Runtime/process interop points (if any) in `src/runtime.rs` / stdlib interop modules

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Ensure dump-mode flags are mutually exclusive and error messaging is deterministic.
2. Ensure dump output serialization failures produce deterministic diagnostics.
3. Harden verify threshold enforcement for cold/warm startup and RSS gates.
4. Harden verify evidence validation paths for missing/invalid manual evidence payloads.
5. If external command execution paths exist, add timeout policies and timeout diagnostics.
6. Keep behavior deterministic in CI by avoiding sleep-race based assertions.
7. Add/extend integration tests for each failure class.

## Dependencies
- `src/main.rs` verify/check handlers
- `src/acceptance.rs` benchmark/evidence loading
- Existing dump and verify tests in `tests/`

## Implementation Approach
1. Consolidate dump-mode argument validation into a single deterministic branch.
2. Add explicit tests for serialization/validation failure behavior.
3. Strengthen verify threshold checks and reporting contracts.
4. Add timeout wrappers for external command paths where applicable.
5. Verify deterministic execution in repeated CI-style test runs.

## Acceptance Criteria

1. **Dump Mode Validation Stability**
   - Given invalid dump-flag combinations
   - When `tonic check` runs
   - Then command fails with deterministic usage diagnostics

2. **Dump Output Contract Stability**
   - Given valid dump mode requests
   - When output is emitted
   - Then stdout format remains stable and test-verified

3. **Benchmark Gate Enforcement**
   - Given benchmark metrics above thresholds
   - When verify runs
   - Then verify fails deterministically with explicit threshold diagnostics

4. **Manual Evidence Gate Enforcement**
   - Given required manual evidence is missing or malformed
   - When verify mixed/manual modes run
   - Then verify fails with deterministic evidence diagnostics

5. **Timeout Safety (If Interop Exists)**
   - Given an external command path that exceeds timeout
   - When invoked via runtime command flow
   - Then execution is terminated and reported with clear timeout diagnostics

## Metadata
- **Complexity**: Medium
- **Labels**: Verify, Dump Modes, Benchmarks, Timeout, Reliability
- **Required Skills**: CLI validation, structured diagnostics, benchmark gate wiring, deterministic integration testing