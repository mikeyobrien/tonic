---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 07. Interactive Run Mode

## Description/Goal
Implement `tonicctl run` to quickly execute a project entry point without a separate build step.

## Background
Developers need a quick way to execute their applications. `tonicctl run` acts as a shortcut that compiles the project to a temporary executable and immediately runs it, streaming inputs and outputs.

## Technical Requirements
- Compile the primary entry point (defined in manifest or `src/main.tn`) to a temporary location.
- Immediately launch the binary in the foreground, inheriting stdin, stdout, and stderr.

## Dependencies
- 03. Core Compiler Integration: tonic compile <path>

## Implementation Approach
- Parse arguments for `run` mode.
- Execute `tonic compile src/main.tn --out target/tmp/app`.
- If successful, swap process or execute `target/tmp/app` synchronously.

## Acceptance Criteria
- Running `tonicctl run` behaves as if compiling and executing in a single step.
- Interactive input streams perfectly cleanly to the application.

## Verification
- Create an app that prompts for input. Use `tonicctl run` to verify stdin operates correctly.

## Suggested Commit
`feat(tonicctl): implement interactive run mode`

## Metadata
- Component: tonicctl
- Type: Feature
