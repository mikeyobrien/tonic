---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement tonicctl CLI argument dispatch and command contract

## Description/Goal
Convert tonicctl from static plan output to command-dispatch behavior with deterministic usage/runtime exits.

## Background
Current example always emits one map payload and does not parse command arguments.

## Technical Requirements
1. Support command forms: `doctor`, `gates`, `bench`, `release --dry-run`.
2. Add options for bench targets (`interpreter|compiled|both`) and strict policy mode.
3. Emit usage errors deterministically for invalid arguments.
4. Keep compile contract wording current (`tonic compile <path> [--out ...]`).

## Dependencies
- Task 05 wrapper API

## Implementation Approach
1. Implement parser + dispatcher module in `examples/apps/tonicctl/src/`.
2. Add structured response model for command outcomes.
3. Add tests for valid/invalid command parsing behavior.

## Acceptance Criteria
- tonicctl can select and execute specific command flows.
- invalid args produce deterministic usage diagnostics.

## Verification
- New tests for tonicctl command parsing contracts.
- `cargo test --test run_tonicctl_meta_example`

## Suggested Commit
`feat(tonicctl): add cli dispatch and usage contract`

## Metadata
- Complexity: Medium
- Labels: tonicctl, cli, contract
