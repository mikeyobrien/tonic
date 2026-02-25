---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement tonicctl release --dry-run workflow and artifact reporting

## Description/Goal
Add release preflight orchestration with safety rails and structured artifact/report output.

## Background
Release flow must enforce clean worktree, doctor, required gates, and strict benchmark policy without tagging/pushing.

## Technical Requirements
1. Enforce `--dry-run` requirement and block unsafe invocation forms.
2. Validate clean git worktree before release flow.
3. Run required release checklist commands and collect results.
4. Write release summary artifacts (json/markdown or deterministic text contract) under configured output dir.

## Dependencies
- Tasks 03, 06, 07, 08
- `docs/release-checklist.md`

## Implementation Approach
1. Build release command pipeline as composable steps.
2. Ensure each step has deterministic fail-fast behavior.
3. Persist summary artifact paths for user/CI consumption.

## Acceptance Criteria
- `tonicctl release --dry-run` enforces all required preflight and gate checks.
- command never performs tag/push operations.
- artifact output is deterministic and references both strict benchmark target reports.

## Verification
- Integration tests covering clean/dirty worktree and gate failure behavior.
- `cargo test -q` passes with new release-flow tests.

## Suggested Commit
`feat(tonicctl): add release dry-run orchestration and reporting`

## Metadata
- Complexity: High
- Labels: tonicctl, release, safety, artifacts
