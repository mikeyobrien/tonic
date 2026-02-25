---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Add host interop process execution primitives with deterministic exit contracts

## Description/Goal
Implement host interop primitives that allow tonicctl to run shell commands and evaluate exit status deterministically.

## Background
tonicctl must execute existing workflow commands (`cargo`, scripts, policy evaluators). Current Tonic language surface lacks process execution in user code.

## Technical Requirements
1. Add host interop key(s) for process execution (e.g. run command string / argv tuple).
2. Return structured results including `exit_code`, `stdout`, `stderr`.
3. Preserve deterministic behavior for command-not-found, spawn failure, and non-zero exit.
4. Keep parity between interpreter and compiled runtime paths.

## Dependencies
- Task 01 capability contract
- Runtime interop modules (`src/interop.rs`, `src/native_runtime/interop.rs`)

## Implementation Approach
1. Implement interop primitive in runtime and native runtime boundary.
2. Add typing/IR/resolver wiring for accepted host_call shape.
3. Add tests for success, non-zero, and missing command behavior.

## Acceptance Criteria
- tonic code can execute process commands through host interop.
- Exit behavior is deterministic in both interpreter and compiled modes.

## Verification
- `cargo test --test runtime_llvm_closures_bindings_interop`
- New targeted tests for process interop success/failure cases.

## Suggested Commit
`feat(interop): add deterministic process execution host primitive`

## Metadata
- Complexity: High
- Labels: tonicctl, interop, process, runtime
