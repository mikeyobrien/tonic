---
status: completed
started: 2026-02-24
completed: 2026-02-25
HEARTBEAT_TASK_STATUS: done
---

# Task: Remove `legacy` For-Op Compile Blocker in LLVM Path

## Goal
Make active `for` comprehension fixtures compile under LLVM backend by eliminating `unsupported instruction legacy` failures.

## Scope
- Replace/bridge `legacy` op usage in native path for `for` forms used by active catalog.
- Ensure compile-time and runtime behavior aligns with catalog contracts.
- Keep known unsupported option behavior deterministic (`for_reduce_fail` should still fail as catalog expects).

## Fixture Targets
- `examples/parity/06-control-flow/for_single_generator.tn`
- `examples/parity/06-control-flow/for_multi_generator.tn`
- `examples/parity/06-control-flow/for_into.tn`
- `examples/parity/06-control-flow/for_into_runtime_fail.tn`

## Deliverables
- LLVM/native lowering support for current active `for` subset.
- Regression tests for compile and direct execution behavior of these fixtures.

## Acceptance Criteria
- Listed fixtures match expected `check_exit` in catalog under LLVM compile.
- Runtime outputs/errors match catalog expectations for listed fixtures.
- `for_reduce_fail` remains deterministic with expected compile failure.

## Verification
- Parity harness confirms compile and run parity for listed fixtures.
- Existing `for` tests remain green.

## Suggested Commit
`feat(llvm): support active for-comprehension forms in native path`
