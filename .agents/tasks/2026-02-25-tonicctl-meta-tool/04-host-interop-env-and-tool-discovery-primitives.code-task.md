---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Add environment and tool-discovery interop primitives

## Description/Goal
Provide the minimum environment/tool discovery surface needed for `tonicctl doctor`.

## Background
Doctor checks require PATH/tool lookup and basic environment visibility. The planner currently hardcodes assumptions.

## Technical Requirements
1. Add host interop support for env lookup and `which`-style command discovery.
2. Return explicit `none`/`err` states for missing keys/tools.
3. Keep deterministic behavior across interpreter and compiled runtime.
4. Avoid adding broad shell-eval behavior in these primitives.

## Dependencies
- Task 01 contract
- Task 02 host interop plumbing

## Implementation Approach
1. Implement env/which interop keys.
2. Add tests for present/missing env vars and tools.
3. Document deterministic result shape for tonicctl consumption.

## Acceptance Criteria
- tonic code can query env and discover tools needed by doctor checks.
- missing values are handled predictably without panics.

## Verification
- New tests for env/which interop.
- `cargo test -q` includes interpreter + compiled path checks.

## Suggested Commit
`feat(interop): add env and tool-discovery primitives`

## Metadata
- Complexity: Medium
- Labels: tonicctl, interop, env, doctor
