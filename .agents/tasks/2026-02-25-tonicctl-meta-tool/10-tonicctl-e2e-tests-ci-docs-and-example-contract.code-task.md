---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Finalize tonicctl with e2e tests, CI coverage, and docs sync

## Description/Goal
Lock tonicctl behavior with broad e2e tests, CI/tooling assertions, and updated docs/example guidance.

## Background
After command implementation, regression protection is required to keep tonicctl and gate scripts aligned over time.

## Technical Requirements
1. Add end-to-end tests for `doctor`, `gates`, `bench --strict`, and `release --dry-run` command contracts.
2. Add CI/doc wiring tests that assert tonicctl references remain consistent with scripts/docs.
3. Update `examples/apps/tonicctl/README.md` and relevant docs with executable usage.
4. Ensure no stale references to legacy compile flags are introduced.

## Dependencies
- Tasks 01–09

## Implementation Approach
1. Build fixture-based command tests for deterministic pass/fail paths.
2. Add docs parity checks similar to existing gate-doc tests.
3. Update example docs to describe planner-vs-executor behavior and constraints.

## Acceptance Criteria
- tonicctl command contracts are covered by deterministic tests.
- docs and tests reflect current dual strict benchmark policy and compile contract.
- full suite remains green.

## Verification
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -q`
- Optional: `./scripts/native-gates.sh`

## Suggested Commit
`test(tonicctl): lock command contracts and docs parity`

## Metadata
- Complexity: Medium
- Labels: tonicctl, testing, ci, docs
