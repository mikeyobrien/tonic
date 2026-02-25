---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Enforce 100% LLVM Catalog Parity Gate in CI

## Goal
Lock in full LLVM parity by enforcing catalog-derived compile/run contracts in local scripts and CI.

## Scope
- Resolve any final residual mismatches after tasks 01–08.
- Add/enable enforce-mode parity gate in CI workflow.
- Ensure reports are archived as artifacts for regression triage.

## Deliverables
- Enforced parity command/script (non-zero on any mismatch).
- CI integration that runs parity gate and uploads parity report artifacts.
- Documentation update for local parity gate usage.

## Acceptance Criteria
1. **Compile Contract Complete**
   - Given active catalog entries
   - When running parity gate
   - Then all entries match `check_exit` under `tonic compile <path>` executable output.

2. **Runtime Contract Complete**
   - Given entries with `check_exit = 0`
   - When executing compiled binaries directly
   - Then `run_exit`, `stdout`, and `stderr_contains` all match catalog expectations.

3. **CI Enforcement**
   - Given pull requests touching frontend/backend/runtime paths
   - When CI runs
   - Then parity gate is enforced and report artifacts are published.

## Verification
- Run parity gate locally in enforce mode (must pass 100%).
- Run `cargo fmt --all`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test`.

## Suggested Commit
`ci(parity): enforce full llvm catalog parity gate`
