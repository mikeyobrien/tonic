---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Extend LLVM Backend for Control Flow, Clauses, and Call Dispatch

## Description
Add control-flow lowering coverage (branches, clause dispatch, guard evaluation, entrypoint behavior) so native code can execute real multi-function programs.

## Background
MVP codegen is insufficient without robust branching/call behavior. This step enables native execution of substantial parity fixtures without interpreter fallback.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/runtime.rs`
- `src/parser.rs`
- `tests/run_function_clauses_defaults_defp_smoke.rs`
- `tests/run_control_forms_smoke.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Lower conditional branches and block terminators to LLVM control flow.
2. Implement function clause selection and guard evaluation lowering strategy.
3. Support module-qualified and local function call resolution through compiled symbols.
4. Preserve deterministic error behavior for no matching clause / arity mismatch.
5. Support main entrypoint invocation for compiled artifact execution.

## Dependencies
- LLVM MVP backend from task 05.
- Native runtime helpers from task 04.
- Existing resolver/type contracts.

## Implementation Approach
1. Add CFG-to-LLVM block mapper and branch terminator emission.
2. Implement clause dispatch scaffolding with helper calls where needed.
3. Add fixture parity tests for if/unless/cond/with and function clauses in native mode.

## Acceptance Criteria

1. **Control Flow Executes Correctly in Native Mode**
   - Given parity fixtures with branches and guard logic
   - When compiled/executed via LLVM backend
   - Then outputs match interpreter mode.

2. **Clause Dispatch Is Deterministic**
   - Given multi-clause functions and guard conditions
   - When invoking compiled functions
   - Then matching behavior and failure diagnostics remain deterministic.

3. **Call Resolution Works Across Modules**
   - Given project programs with module-qualified and local calls
   - When running native artifacts
   - Then call targets resolve and execute correctly.

4. **Unit/Integration Tests Cover Control Flow and Calls**
   - Given codegen and runtime integration changes
   - When running `cargo test`
   - Then tests cover control flow lowering, clause dispatch, and call behavior.

## Metadata
- **Complexity**: High
- **Labels**: LLVM, Control-Flow, Dispatch, Clauses
- **Required Skills**: CFG lowering, dynamic dispatch strategy, diagnostic stability
