---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 05. Formatting Tool Integration

## Description/Goal
Add support for source code formatting via `tonicctl fmt`.

## Background
Consistent code style is essential. As an executable meta-tool, `tonicctl` should orchestrate formatting checks or apply formatting across the entire project structure.

## Technical Requirements
- Iterate over all `.tn` source files in the project workspace (guided by the manifest).
- Invoke the underlying `tonic fmt` for each file or across the workspace.

## Dependencies
- 04. Workspace Manifest Validation

## Implementation Approach
- Traverse directories recursively using `src/` and `tests/`.
- Run the formatting command and track success.
- Report all unformatted files or formatting errors clearly to the user.

## Acceptance Criteria
- `tonicctl fmt` correctly formats all `.tn` files in a workspace.
- Exit code represents if formatting succeeded or failed.

## Verification
- Introduce a misformatted `.tn` file and run `tonicctl fmt`.
- Verify the file is fixed and the meta-tool exits cleanly.

## Suggested Commit
`feat(tonicctl): add formatting tool integration`

## Metadata
- Component: tonicctl
- Type: Feature
