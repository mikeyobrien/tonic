---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Create stdlib-facing System module wrappers for tonicctl v1

## Description/Goal
Add a small Tonic stdlib-style `System` surface that wraps host interop primitives for process/fs/env operations.

## Background
Direct `host_call` everywhere is brittle. tonicctl needs readable, typed wrappers with stable contracts.

## Technical Requirements
1. Add wrapper functions (e.g. `System.run/1`, `System.which/1`, `System.path_exists/1`, `System.write_text/2`).
2. Ensure wrappers normalize return shapes and error values.
3. Keep wrapper behavior deterministic and documented.
4. Keep API minimal to avoid over-scoping v1.

## Dependencies
- Tasks 02–04 interop primitives

## Implementation Approach
1. Implement a module in tonic examples/runtime surface for wrappers.
2. Add integration tests validating wrapper output contracts.
3. Use wrappers in tonicctl instead of raw host_call usage.

## Acceptance Criteria
- tonicctl code no longer depends on low-level host_call formatting details.
- wrapper API is sufficient for all tonicctl commands in this roadmap.

## Verification
- New tests for wrapper success/failure behavior.
- `cargo test -q` passes.

## Suggested Commit
`feat(stdlib): add minimal system wrappers for tonicctl`

## Metadata
- Complexity: Medium
- Labels: tonicctl, stdlib, interop, api
