---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Build LLVM Catalog Parity Harness (Compile + Direct Run)

## Goal
Create a deterministic harness that measures LLVM compiled parity directly against `examples/parity/catalog.toml` and reports progress toward 100%.

## Scope
- Add a parity harness that, for each active catalog entry:
  1. runs `tonic compile <path> --backend llvm`
  2. verifies compile exit against `check_exit`
  3. for successful compile entries, runs compiled executable directly (`./artifact`)
  4. validates `run_exit`, `stdout`, and `stderr_contains` expectations from catalog
- Emit machine-readable and markdown parity reports.
- Support `--enforce` mode to fail on any mismatch.

## Out of Scope
- Fixing lowering/runtime behavior itself.
- Changing catalog expectations.

## Deliverables
- New harness (test or script) under repo tooling.
- Report artifacts (JSON + Markdown) with per-fixture mismatch reasons.
- Summary counters:
  - compile expectation matches/mismatches
  - runtime parity matches/mismatches

## Acceptance Criteria
- A single command produces a deterministic parity report from the catalog.
- Report includes fixture-level reasons and top grouped failure causes.
- Enforce mode exits non-zero when parity is not complete.

## Verification
- Run harness in non-enforce mode and confirm report generated.
- Run harness in enforce mode and confirm failure while gaps remain.

## Suggested Commit
`feat(parity): add llvm catalog parity harness and report`
