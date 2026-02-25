---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Define system capability contract and safety model for tonicctl

## Description/Goal
Define the minimum system capabilities tonicctl needs and the safety boundaries for exposing them to Tonic code.

## Background
The planner example cannot execute shell commands, inspect environment, or write artifacts. We need a deliberate contract before adding host interop primitives.

## Technical Requirements
1. Specify required capability groups: process execution, filesystem I/O, environment/tool discovery.
2. Define deterministic success/failure result shape for each capability.
3. Define safety rails for release workflows (`--dry-run` enforcement, no implicit tag/push).
4. Define deterministic error categories and expected non-zero exits.

## Dependencies
- Existing planner example: `examples/apps/tonicctl`
- Existing interop pattern: `src/interop.rs`, `src/native_runtime/interop.rs`

## Implementation Approach
1. Add a design note in task docs and map each tonicctl command to required capability calls.
2. Define a typed data contract for command outcomes and failure reasons.
3. Lock a v1 capability list (no extra scope).

## Acceptance Criteria
- Capability contract is explicit and complete for `doctor`, `gates`, `bench --strict`, `release --dry-run`.
- Safety constraints are documented and testable.

## Verification
- Design doc review + command/capability matrix included in implementation notes.
- No command relies on unspecified runtime behavior.

## Suggested Commit
`design(tonicctl): define system capability and safety contract`

## Metadata
- Complexity: Medium
- Labels: tonicctl, interop, safety, contract
