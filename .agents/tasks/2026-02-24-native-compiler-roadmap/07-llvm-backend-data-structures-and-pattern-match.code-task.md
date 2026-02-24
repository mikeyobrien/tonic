---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Add LLVM Lowering for Collections and Pattern Matching

## Description
Implement native lowering support for tuples/lists/maps/keywords and pattern matching semantics (`case`, list/map patterns, pin, match operator).

## Background
Pattern matching and collection manipulation are core language identity features. Native mode is not viable until these semantics are available and equivalent to interpreter behavior.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/runtime.rs`
- `src/ir.rs`
- `tests/run_case_list_map_smoke.rs`
- `tests/run_case_pin_guard_match_smoke.rs`
- `tests/run_collections_smoke.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Lower collection constructors and access/update operations through runtime ABI helpers.
2. Implement compiled pattern matching flow for tuple/list/map forms.
3. Support pin operator and match operator semantics in compiled mode.
4. Preserve deterministic failure behavior for bad matches and non-exhaustive cases.
5. Ensure map/list-heavy programs execute without interpreter fallback.

## Dependencies
- Runtime value ABI and helper library (tasks 03–04).
- LLVM control-flow support (task 06).
- Existing pattern-matching parser/IR contracts.

## Implementation Approach
1. Implement codegen helpers for container construction and extraction.
2. Build pattern-match lowering utilities reusable across case/clauses/match op.
3. Add differential tests for collection and pattern fixtures under native backend.

## Acceptance Criteria

1. **Collections Behave Correctly in Native Mode**
   - Given programs constructing and using tuples/lists/maps/keywords
   - When compiled/executed via LLVM backend
   - Then outputs match interpreter mode.

2. **Pattern Matching Semantics Are Preserved**
   - Given fixtures with list/map/pin/guard patterns and match operator
   - When executed in native mode
   - Then branch selection and bindings match interpreter semantics.

3. **Mismatch Diagnostics Remain Deterministic**
   - Given bad-match and non-exhaustive scenarios
   - When running native artifacts
   - Then deterministic error contracts are preserved.

4. **Unit/Integration Tests Cover Collections and Patterns**
   - Given backend and runtime pattern logic
   - When running `cargo test`
   - Then tests cover positive/negative cases across collection and pattern features.

## Metadata
- **Complexity**: High
- **Labels**: LLVM, Collections, Pattern-Matching, Semantics
- **Required Skills**: Dynamic value manipulation, pattern compiler design, test parity
