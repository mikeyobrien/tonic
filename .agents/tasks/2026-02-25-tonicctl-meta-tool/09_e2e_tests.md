---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 09. Meta-Tool End-to-End Test Suite

## Description/Goal
Author comprehensive tests covering the meta-tool's executable distribution and workflow orchestration.

## Background
The meta-tool serves as a central hub for all development tasks. It must be resilient against regressions across its command suite.

## Technical Requirements
- Write integration tests executing the compiled `tonicctl` binary against mock workspaces.
- Test commands: `build`, `run`, `test`, `fmt`.

## Dependencies
- 08. Build Artifact Caching Strategy

## Implementation Approach
- Use shell scripts or a tonic test harness to set up temporary directories with source code.
- Run the compiled `tonicctl` against these directories.
- Assert expected file creations, output streams, and status codes.

## Acceptance Criteria
- A dedicated E2E test suite validates all primary meta-tool workflows.
- Integration tests respect the dual strict gates policy and never reference legacy backend flags.

## Verification
- Run the newly created test suite.
- Ensure 100% pass rate.

## Suggested Commit
`test(tonicctl): add meta-tool end-to-end test suite`

## Metadata
- Component: tonicctl
- Type: Test
