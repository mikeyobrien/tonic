---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement tonicctl gates and bench --strict command execution

## Description/Goal
Make tonicctl execute native gate and strict benchmark policy flows (interpreter and compiled targets).

## Background
Planner output lists commands but does not run them or interpret policy verdict outcomes.

## Technical Requirements
1. Implement `tonicctl gates` to run the repo native gate chain deterministically.
2. Implement `tonicctl bench --strict --target <...>` for interpreter/compiled/both.
3. Evaluate strict policy commands and fail deterministically on non-pass verdicts.
4. Preserve dual strict benchmark behavior introduced in current repo scripts.

## Dependencies
- Tasks 02–07

## Implementation Approach
1. Execute known script commands via process wrapper.
2. Thread env overrides and artifact output paths for each target.
3. Normalize command output into tonicctl summary model.

## Acceptance Criteria
- tonicctl can run strict bench flow for interpreter and compiled targets.
- non-pass policy verdicts return non-zero with actionable diagnostics.

## Verification
- Integration tests with fixture scripts for pass/fail policy verdict handling.
- `cargo test -q` and relevant gate-doc tests remain green.

## Suggested Commit
`feat(tonicctl): execute gates and strict benchmark policy flows`

## Metadata
- Complexity: High
- Labels: tonicctl, gates, benchmark, strict-policy
