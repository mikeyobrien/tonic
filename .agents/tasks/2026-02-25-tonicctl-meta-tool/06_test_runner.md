---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 06. Test Runner Orchestration

## Description/Goal
Add support for running project tests using `tonicctl test`.

## Background
A full meta-tool needs a testing story. It must discover tests under `tests/` and run them by compiling and executing them.

## Technical Requirements
- Discover all `.tn` files inside the `tests/` directory.
- Compile them natively via `tonic compile <test_file>`.
- Execute the resulting binaries and track their exit status to determine test success or failure.

## Dependencies
- 04. Workspace Manifest Validation
- 03. Core Compiler Integration: tonic compile <path>

## Implementation Approach
- Leverage file discovery from the manifest tasks.
- For each test file, orchestrate a build step, and if successful, invoke the resulting executable.
- Aggregate test results and emit a final report summary.

## Acceptance Criteria
- Running `tonicctl test` discovers, builds, and runs all test suites.
- Any failing test binary fails the entire test suite run with a non-zero exit code.

## Verification
- Add a failing dummy test. Verify `tonicctl test` exits `1` and prints the output.
- Fix the test, verify `tonicctl test` exits `0`.

## Suggested Commit
`feat(tonicctl): implement test runner orchestration`

## Metadata
- Component: tonicctl
- Type: Feature
