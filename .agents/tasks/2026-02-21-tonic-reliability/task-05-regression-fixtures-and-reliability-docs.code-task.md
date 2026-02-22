# Task: Add Reliability Regression Matrix, Fixtures, and Docs

## Description
Create a consolidated reliability regression layer that codifies high-value failure scenarios, ensures repeatable test coverage, and documents command contracts and operational guarantees for Tonic users.

## Background
Reliability hardening only sticks if the team has an explicit regression matrix and clear docs. This task packages outcomes from Tasks 01–04 into maintainable fixtures and documentation so behavior remains stable under future feature growth.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Reliability umbrella task: `.agents/tasks/2026-02-21-tonic-reliability/harden-cli-runtime-reliability.code-task.md`
- Prior split tasks in this folder (`task-01` through `task-04`)
- Existing integration suite in `tests/`
- CLI and runtime docs (`AGENTS.md`, project docs as applicable)

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Add a reliability regression matrix document mapping failure classes to tests.
2. Consolidate and normalize fixture helpers for temp project creation and failure injection.
3. Ensure regression suite covers:
   - command contract errors
   - manifest/loader failures
   - cache/artifact corruption and conflict
   - verify/dump failure paths
4. Add documentation for deterministic command contracts and non-goals (no OTP runtime in v0).
5. Add contributor guidance for adding new command/features without breaking contracts.
6. Ensure test names and fixture IDs are stable and debuggable.

## Dependencies
- Prior reliability implementation from Tasks 01–04
- Integration test suite in `tests/`
- Project docs (`AGENTS.md` and/or new reliability doc path)

## Implementation Approach
1. Create a `reliability-matrix` markdown artifact under project docs or `.agents/planning`.
2. Refactor duplicate fixture setup helpers into shared test utilities where appropriate.
3. Add/organize integration tests so failure classes are easy to locate.
4. Document command output/exit contracts and extension rules for future commands.

## Acceptance Criteria

1. **Reliability Matrix Exists and Maps to Tests**
   - Given known reliability failure classes
   - When maintainers inspect docs
   - Then each class maps to concrete test files/cases

2. **Fixture Reuse Improves Maintainability**
   - Given repeated project setup patterns across tests
   - When the suite is refactored
   - Then fixture duplication is reduced without losing readability

3. **Coverage of Critical Failure Classes**
   - Given the hardened reliability scope
   - When regression suite executes
   - Then command, loader, cache, and verify failure classes are covered

4. **Contributor Guidance Prevents Contract Drift**
   - Given a future command behavior change
   - When contributors follow docs
   - Then they know which contract tests must be updated and why

5. **Suite Stability**
   - Given repeated local/CI test execution
   - When regression suite runs repeatedly
   - Then tests remain deterministic and non-flaky

## Metadata
- **Complexity**: Low
- **Labels**: Reliability, Regression, Test Infrastructure, Documentation, Maintainability
- **Required Skills**: Test architecture, documentation quality, fixture design, CI reliability practices
## Status
HEARTBEAT_TASK_STATUS=done

