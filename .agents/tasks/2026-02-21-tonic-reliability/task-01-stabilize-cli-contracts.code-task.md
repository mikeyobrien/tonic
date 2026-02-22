> **Status:** Completed
> **Completed By:** gemini-coder
> **Completed At (UTC):** 2026-02-22T04:40:40Z
> **Completed Commit:** df109f75f578068e597a79cff77fe09d8723a600
HEARTBEAT_TASK_STATUS=done

# Task: Stabilize CLI Command Contracts and Diagnostics

## Description
Standardize Tonic command behavior contracts so each command has deterministic argument validation, stderr/stdout behavior, and exit-code semantics. This task hardens user-facing reliability for `run`, `check`, `test`, `fmt`, `verify`, and `compile` (if present).

## Background
Tonic’s value for scripting workflows depends on predictable automation behavior. CI and shell users need stable contracts for failures and machine parsing. Inconsistent messaging or exit codes causes brittle scripts and support churn.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Scope and non-goals: `.agents/planning/2026-02-20-elixir-tui-cli-language/idea-honing.md`
- CLI routing and handlers: `src/main.rs`
- Diagnostic contract: `src/cli_diag.rs`
- Existing CLI tests: `tests/cli_help_smoke.rs`, `tests/check_test_fmt_command_paths.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define a command contract table (usage errors vs runtime errors vs success) for all top-level commands.
2. Ensure usage errors consistently return the usage exit code and print usage-style diagnostics.
3. Ensure runtime/validation errors consistently return failure exit code and print `error: ...` diagnostics.
4. Ensure success paths only print expected stdout output for that command.
5. Normalize argument parsing behavior for unexpected extra args and missing required args.
6. Keep command-specific help output deterministic (`--help` and top-level `--help`).
7. Add integration tests for each contract branch.

## Dependencies
- `src/main.rs`
- `src/cli_diag.rs`
- Existing CLI integration test patterns in `tests/`

## Implementation Approach
1. Add a small internal contract matrix (doc comment or test fixture) for command outcomes.
2. Refactor handler branches to use `CliDiagnostic` consistently.
3. Add/extend integration tests that assert exact stdout/stderr/exit status behavior.
4. Verify no contract regressions for existing green scenarios.

## Acceptance Criteria

1. **Usage Error Consistency**
   - Given any command invocation with invalid argument shape
   - When Tonic exits
   - Then the exit code and stderr follow a deterministic usage-error contract

2. **Runtime Error Consistency**
   - Given a valid command shape with invalid runtime input (missing file, bad manifest)
   - When Tonic exits
   - Then the exit code and stderr follow a deterministic runtime-error contract

3. **Success Output Stability**
   - Given valid command invocations
   - When commands succeed
   - Then stdout only contains the documented command output and stderr is empty

4. **Automated Contract Coverage**
   - Given the updated CLI handlers
   - When `cargo test` runs
   - Then command contract tests pass and guard against future drift

## Metadata
- **Complexity**: Medium
- **Labels**: CLI, Diagnostics, Exit Codes, Reliability, Testing
- **Required Skills**: Rust CLI parsing, error handling, integration test design