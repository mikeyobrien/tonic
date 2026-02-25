---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 10. Documentation Update and Polish

## Description/Goal
Finalize user and developer documentation for `tonicctl`, describing its usage and transition to an executable meta-tool.

## Background
Now that `tonicctl` operates as a compiled standalone workflow orchestrator rather than just a planner, documentation must reflect the latest executable usage.

## Technical Requirements
- Update `examples/apps/tonicctl/README.md`.
- Document commands, arguments, and expected behavior.
- Ensure absolute clarity that `tonic compile <path>` is the engine under the hood.

## Dependencies
- 09. Meta-Tool End-to-End Test Suite

## Implementation Approach
- Audit existing docs for pure-planner verbiage and replace with executable meta-tool terminology.
- Generate standard CLI help text.
- Verify no outdated flags exist in any documentation snippet.

## Acceptance Criteria
- Documentation is accurate, comprehensive, and cleanly formatted.

## Verification
- Read through generated documentation for clarity and style conformance.
- Validate examples run accurately.

## Suggested Commit
`docs(tonicctl): polish executable meta-tool documentation`

## Metadata
- Component: tonicctl
- Type: Documentation
