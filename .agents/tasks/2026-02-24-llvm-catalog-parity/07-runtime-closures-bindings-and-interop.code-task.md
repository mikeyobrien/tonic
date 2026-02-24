---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement Closure/Binding/Interop Runtime Helpers in Native Path

## Goal
Eliminate remaining function-value, binding load, and host interop parity gaps for compiled execution.

## Scope
- Implement/enable native helpers and call paths for:
  - closure creation/invocation (`tn_runtime_make_closure`, related call-value path)
  - binding load semantics (`tn_runtime_load_binding`)
  - host call/protocol dispatch behavior for active interop fixture
- Preserve deterministic error behavior for unsupported host operations.

## Fixture Targets
- `examples/parity/05-functions/anonymous_fn_capture_invoke.tn`
- `examples/parity/06-control-flow/cond_branches.tn` (binding path)
- `examples/parity/08-errors/host_call_and_protocol_dispatch.tn`
- any additional closure/binding/interop fixtures identified by parity harness

## Deliverables
- Native implementations replacing current stub-abort behavior for these helper paths.
- Regression tests for closure calls, binding loads, and host interop in compiled mode.

## Acceptance Criteria
- Listed fixtures run successfully in compiled mode with catalog-matching output/exit.
- No `native runtime not available` errors for closure/binding/interop helper paths.

## Verification
- Parity harness helper-gap buckets for closure/binding/interop are zero.
- Existing interop tests remain green.

## Suggested Commit
`feat(native): implement closure binding and interop helpers for compiled runtime`
