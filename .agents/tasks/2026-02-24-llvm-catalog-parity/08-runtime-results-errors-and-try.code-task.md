---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Result/Error/Try Semantic Parity in Compiled Runtime

## Goal
Match interpreter error-flow semantics for `ok/err`, `?`, `try/rescue/catch/after`, and related runtime diagnostics.

## Scope
- Implement native helper behavior for:
  - `tn_runtime_make_ok`
  - `tn_runtime_make_err`
  - `tn_runtime_try`
  - question-operator and raise propagation paths used by active fixtures
- Ensure stderr messages satisfy catalog `stderr_contains` contracts.

## Fixture Targets
- `examples/ergonomics/error_propagation.tn`
- `examples/parity/08-errors/question_operator_success.tn`
- `examples/parity/08-errors/question_operator_err_bubble.tn`
- `examples/parity/08-errors/try_rescue_success.tn`
- `examples/parity/08-errors/try_catch_success.tn`
- `examples/parity/08-errors/try_after_success.tn`
- `examples/parity/08-errors/try_rescue_catch_after_success.tn`

## Deliverables
- Native error/result helper implementations aligned with interpreter semantics.
- Regression tests for positive and failing error-flow paths in compiled mode.

## Acceptance Criteria
- Listed fixtures match catalog run exit/stdout/stderr expectations in compiled mode.
- No helper-stub aborts for result/try helper paths.

## Verification
- Parity harness reports run parity for listed fixtures.
- Existing error-semantics tests remain green.

## Suggested Commit
`fix(native): align compiled result and try semantics with interpreter`
