---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement tonicctl doctor command with real runtime checks

## Description/Goal
Execute actual doctor checks (tools/files/writable dirs) and produce deterministic pass/fail output.

## Background
Doctor is currently a static plan. We need real checks before running gates/release flows.

## Technical Requirements
1. Validate required commands (`cargo`, `rustc`, `python3`, `cc|gcc|clang`).
2. Validate required files/scripts/manifests for current repo.
3. Validate writable artifact directories.
4. Produce machine-readable status + deterministic non-zero on blockers.

## Dependencies
- Tasks 04–06

## Implementation Approach
1. Implement doctor evaluator using System wrappers.
2. Group results by required/warn severity.
3. Add tests for success and failure scenarios (missing command/file).

## Acceptance Criteria
- doctor reports actionable status and blocks when required checks fail.
- output contract is stable across interpreter and compiled runs.

## Verification
- New `tests/run_tonicctl_doctor_*` integration tests.
- `cargo test -q` passes.

## Suggested Commit
`feat(tonicctl): implement doctor runtime checks`

## Metadata
- Complexity: Medium
- Labels: tonicctl, doctor, reliability
