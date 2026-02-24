---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Match/Case/Guard Runtime Parity for Compiled Execution

## Goal
Bring compiled pattern-matching and guarded control-flow behavior to parity with interpreter results in catalog fixtures.

## Scope
- Ensure native runtime + generated code correctly evaluate:
  - `case` matching over atom/list/tuple/map patterns
  - pin patterns and guards
  - `match` operator behavior
  - `cond`/`with` branch semantics used in active fixtures
- Preserve deterministic mismatch/failure diagnostics where expected.

## Fixture Targets
- `examples/parity/04-patterns/case_atom_and_wildcard.tn`
- `examples/parity/04-patterns/case_list_bind.tn`
- `examples/parity/04-patterns/case_map_arrow_pattern.tn`
- `examples/parity/04-patterns/case_tuple_bind.tn`
- `examples/parity/04-patterns/match_operator_bindings.tn`
- `examples/parity/04-patterns/pin_pattern_and_guard.tn`
- `examples/parity/06-control-flow/cond_branches.tn`
- `examples/parity/06-control-flow/with_happy_path.tn`
- `examples/parity/06-control-flow/with_else_fallback.tn`

## Deliverables
- Runtime/codegen parity fixes for pattern/guard and branch behavior.
- Regression tests covering each pattern/control-flow category above.

## Acceptance Criteria
- Listed fixtures produce exact catalog-conformant runtime results in compiled mode.
- No helper-stub aborts or branch-dispatch mismatches for covered patterns.

## Verification
- Parity harness reports listed fixtures as run-parity pass.
- Existing pattern/control-flow tests remain green.

## Suggested Commit
`fix(native): align pattern guard and branch runtime semantics with interpreter`
