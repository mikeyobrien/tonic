---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Implement Native Runtime Core Value Constructors (Atoms/Tuples/Lists/Range)

## Goal
Close the largest runtime helper gap bucket by implementing core constructors currently aborting in compiled binaries.

## Scope
- Implement native runtime helpers used by compiled code:
  - `tn_runtime_const_atom`
  - `tn_runtime_make_tuple`
  - `tn_runtime_make_list`
  - `tn_runtime_range`
  - map/keyword constructors + access/update paths used by active fixtures
- Ensure rendered output matches interpreter formatting for active catalog expectations.

## Fixture Targets
High-impact fixtures currently failing due these helpers, including:
- `examples/parity/01-literals/atom_expression.tn`
- `examples/parity/02-operators/arithmetic_basic.tn`
- `examples/parity/02-operators/comparison_set.tn`
- `examples/parity/02-operators/membership_and_range.tn`
- `examples/parity/03-collections/list_literal.tn`
- `examples/parity/03-collections/tuple_literal_and_match.tn`
- `examples/parity/03-collections/keyword_literal_single_entry.tn`
- `examples/parity/03-collections/map_literal_single_entry.tn`
- `examples/parity/03-collections/map_update_single_key.tn`
- `examples/parity/03-collections/map_dot_and_index_access.tn`
- `examples/parity/99-stretch/list_cons_pattern.tn`
- `examples/parity/99-stretch/bitstring_binary.tn`
- `examples/parity/99-stretch/multi_entry_map_literal.tn`
- `examples/parity/99-stretch/multi_entry_keyword_literal.tn`
- plus other atom/list/tuple/map fixtures flagged by parity harness

## Deliverables
- Native runtime helper implementations (no stub-abort for listed helpers).
- Regression tests validating direct executable output parity for these value types.

## Acceptance Criteria
- No parity failures remain whose primary reason is missing helpers listed above.
- Direct executable output matches catalog outputs for covered fixtures.

## Verification
- Parity harness grouped failures show zero for listed helper names.
- Targeted runtime/collection tests pass.

## Suggested Commit
`feat(native): implement atom tuple list and range runtime helpers`
