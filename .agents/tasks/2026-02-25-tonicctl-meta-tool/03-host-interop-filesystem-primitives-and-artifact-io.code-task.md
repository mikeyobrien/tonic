---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Add filesystem host interop primitives for checks and artifact I/O

## Description/Goal
Expose minimal filesystem primitives required for tonicctl doctor/release/report flows.

## Background
tonicctl needs file existence checks, directory creation, and deterministic report writing for native gate artifacts.

## Technical Requirements
1. Add host interop for path existence checks (`file`/`dir`), directory creation, and text write.
2. Return deterministic errors for permission denied, invalid path, and write failures.
3. Keep behavior stable across interpreter and compiled execution.
4. Ensure primitives are path-safe and do not silently ignore failures.

## Dependencies
- Task 01 capability contract
- Task 02 process execution interop conventions

## Implementation Approach
1. Implement filesystem interop keys and shared error mapping.
2. Add runtime/native-runtime tests with fixture temp directories.
3. Add contract tests for deterministic diagnostics.

## Acceptance Criteria
- tonic code can validate required files/dirs and write report artifacts.
- failures produce deterministic, actionable diagnostics.

## Verification
- New tests under `tests/` for fs interop success/failure.
- `cargo test -q` remains green.

## Suggested Commit
`feat(interop): add filesystem primitives for tonicctl artifacts`

## Metadata
- Complexity: High
- Labels: tonicctl, interop, filesystem, artifacts
