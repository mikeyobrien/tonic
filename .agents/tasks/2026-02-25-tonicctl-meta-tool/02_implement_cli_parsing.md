---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 02. Implement CLI Argument Parsing

## Description/Goal
Implement parsing for `tonicctl` subcommands like `build`, `test`, `fmt`, and their respective arguments.

## Background
To act as a meta-tool, `tonicctl` needs to understand user intentions (which subcommand is executed) and extract parameters like paths or flags (e.g., `--out`).

## Technical Requirements
- Support parsing top-level commands: `build`, `run`, `fmt`, `test`.
- Extract path arguments and output flags.
- Do NOT include legacy `--backend` compile flags.

## Dependencies
- 01. Setup Meta-Tool Project Structure

## Implementation Approach
- Add a parsing module that matches against process arguments.
- Define data structures (e.g., enums or dicts) representing the active subcommand.
- Implement basic validation of input paths.

## Acceptance Criteria
- Running `tonicctl` with a subcommand outputs the parsed representation.
- Missing or invalid arguments fail gracefully with a usage message.

## Verification
- Pass `examples/apps/tonicctl` mock arguments to the parser and verify expected outputs.
- Adhere strictly to dual strict gates for correctness.

## Suggested Commit
`feat(tonicctl): implement CLI argument parsing`

## Metadata
- Component: tonicctl
- Type: Feature
