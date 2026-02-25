---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# 04. Workspace Manifest Validation

## Description/Goal
Integrate parsing and validation of the `tonic.toml` workspace manifest inside the meta-tool.

## Background
A robust meta-tool must be aware of workspace constraints, package names, dependencies, and configuration. Reading the `tonic.toml` manifest enables commands like `test` or `fmt` to apply configuration globally.

## Technical Requirements
- Parse `tonic.toml` inside the target directory.
- Extract basic metadata (package name, entry points).
- Ensure missing manifests are handled with clear diagnostics.

## Dependencies
- 01. Setup Meta-Tool Project Structure

## Implementation Approach
- Add a manifest parsing routine.
- Surface semantic validation errors if `tonic.toml` is malformed.
- Cache or expose manifest metadata to subsequent phases (build/test/fmt).

## Acceptance Criteria
- `tonicctl` fails informatively if run without a target or if `tonic.toml` is invalid.
- Metadata is successfully extracted and printed in verbose modes.

## Verification
- Run `tonicctl` in directories without a manifest and ensure it correctly diagnoses the missing configuration.

## Suggested Commit
`feat(tonicctl): integrate workspace manifest validation`

## Metadata
- Component: tonicctl
- Type: Feature
