---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 03. Core Compiler Integration: tonic compile <path>

## Description/Goal
Implement the meta-tool's ability to orchestrate compilation natively by invoking the underlying Tonic compiler commands.

## Background
The meta-tool's `build` command needs to translate to standard `tonic compile <path> [--out ...]` under the hood. No legacy backend arguments should be passed.

## Technical Requirements
- Map `tonicctl build <path>` to system/process invocation of `tonic compile <path> [--out ...]`.
- Handle output binaries seamlessly.
- Bubble up any compile-time diagnostics or failure codes to the user.

## Dependencies
- 02. Implement CLI Argument Parsing

## Implementation Approach
- Use standard library APIs to launch child processes.
- Forward standard output and error output to the terminal.
- Wait for exit status to determine success.

## Acceptance Criteria
- Running `tonicctl build` successfully invokes the `tonic compile` command and produces an executable output.

## Verification
- Compile a sample test file using `tonicctl build examples/apps/tonicctl/main.tn`.
- Validate binary executes properly.

## Suggested Commit
`feat(tonicctl): orchestrate core compiler invocation`

## Metadata
- Component: tonicctl
- Type: Feature
