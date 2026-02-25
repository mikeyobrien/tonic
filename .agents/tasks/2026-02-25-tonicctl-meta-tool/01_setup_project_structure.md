---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 01. Setup Meta-Tool Project Structure

## Description/Goal
Initialize the project structure for `examples/apps/tonicctl` to prepare for its evolution from a pure planner into an executable meta-tool.

## Background
Currently, `examples/apps/tonicctl` exists as a pure planner or script. We need to restructure it as a full Tonic project capable of being compiled to a native executable via `tonic compile <path>`.

## Technical Requirements
- Define a clear module hierarchy for CLI parsing, subcommands, and compiler invocation.
- Setup a `tonic.toml` manifest for the tool if needed, or ensure the directory layout follows tonic conventions.

## Dependencies
- None

## Implementation Approach
- Create directories for subcommands under `examples/apps/tonicctl/src/`.
- Ensure a clear entry point `main.tn` exists.
- Stub out modules for core functionalities (build, test, fmt).

## Acceptance Criteria
- Project structure reflects a standard, multi-module Tonic application.
- `tonic compile examples/apps/tonicctl/src/main.tn` succeeds as a baseline check.

## Verification
- Run `tonic compile examples/apps/tonicctl/src/main.tn` to verify no syntactic errors in stubs.
- Ensure dual strict gates are respected.

## Suggested Commit
`feat(tonicctl): initialize meta-tool project structure`

## Metadata
- Component: tonicctl
- Type: Refactor
