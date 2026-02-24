---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Implement Native Error Semantics (`ok/err`, `?`, `try/rescue/catch/after`, `raise`)

## Description
Add full compiled-mode support for Tonic’s result-first and exception-like control flows, including deterministic propagation and handler behavior.

## Background
Rust/Go-class runtime performance is irrelevant if the most important control-flow semantics differ by backend. Error behavior must be semantically equivalent before optimization.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/runtime.rs`
- `tests/run_result_propagation.rs`
- `tests/run_try_raise_smoke.rs`
- `tests/check_try_raise_typing.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Implement compiled lowering for result constructors and question-propagation behavior.
2. Implement lowering/runtime support for `try/rescue/catch/after` forms.
3. Implement compiled `raise` semantics with deterministic raised-value handling.
4. Preserve existing exit-code and diagnostic behavior for unhandled runtime failures.
5. Ensure interaction between result and exception flows remains coherent and deterministic.

## Dependencies
- LLVM backend support from tasks 05–07.
- Runtime ABI and helper library from tasks 03–04.
- Existing typing and diagnostic contracts.

## Implementation Approach
1. Encode error-flow edges in MIR/LLVM lowering with explicit handler blocks.
2. Delegate complex dynamic matching to runtime helper calls where necessary.
3. Add differential fixtures for all error-flow combinations.

## Acceptance Criteria

1. **Question Propagation Works Natively**
   - Given functions using `ok/err` and `?`
   - When compiled and executed
   - Then success and bubbling behavior match interpreter mode.

2. **Try/Rescue/Catch/After Semantics Match**
   - Given fixtures covering rescue, catch, and after combinations
   - When run in native mode
   - Then handler and finalization behavior matches interpreter semantics.

3. **Unhandled Failures Preserve Contracts**
   - Given unhandled raise/error paths
   - When executing native artifacts
   - Then deterministic failure output and exit status are preserved.

4. **Unit/Integration Tests Cover Error Semantics**
   - Given compiled error-flow implementation
   - When running `cargo test`
   - Then tests cover success/failure propagation and mixed control-flow cases.

## Metadata
- **Complexity**: High
- **Labels**: LLVM, Error-Handling, Result-Propagation, Runtime-Semantics
- **Required Skills**: Exception/control-flow lowering, diagnostic compatibility, runtime design
