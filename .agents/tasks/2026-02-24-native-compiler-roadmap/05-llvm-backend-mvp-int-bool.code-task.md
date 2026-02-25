---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Implement LLVM Backend MVP for Integer/Boolean Programs

## Description
Ship the first working LLVM backend path for a strict subset (integer/bool literals, arithmetic, comparisons, direct function calls, returns).

## Background
A small, end-to-end MVP de-risks toolchain, artifact emission, and backend architecture before tackling dynamic features and advanced control flow.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/bin/benchsuite.rs`
- `src/main.rs`
- `src/ir.rs`
- `research/track5-tooling-developer-workflow.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Add LLVM backend crate/module wiring (`inkwell` or equivalent) with pinned LLVM compatibility.
2. Lower MIR subset (consts, add/sub/mul/div, cmp, call, return) to LLVM IR.
3. Emit `.ll` and `.o` artifacts from `tonic compile`.
4. Ensure compile command routes through native backend path for MVP subset programs.
5. Provide graceful unsupported-op diagnostics for out-of-subset programs.

## Dependencies
- MIR layer from task 02.
- Runtime ABI from task 03 (minimal subset use).
- CLI compile pipeline in `src/main.rs`.

## Implementation Approach
1. Build standalone codegen entrypoint for MIR function compilation.
2. Add deterministic LLVM IR text emission tests for known fixtures.
3. Wire compile command and artifact naming conventions for backend mode.

## Acceptance Criteria

1. **LLVM MVP Compiles Subset Correctly**
   - Given subset-compatible programs
   - When compiling with `tonic compile`
   - Then `.ll`/`.o` artifacts are produced successfully.

2. **Unsupported Features Fail Deterministically**
   - Given programs outside the MVP subset
   - When compiling with LLVM backend
   - Then compilation fails with actionable unsupported-op diagnostics.

3. **CLI Compile Contract Works for MVP Path**
   - Given compile command invocations
   - When `tonic compile` is executed on subset fixtures
   - Then native backend artifacts are produced with deterministic diagnostics.

4. **Unit/Integration Tests Cover LLVM MVP Path**
   - Given LLVM codegen modules
   - When running `cargo test`
   - Then tests cover lowering correctness, artifact emission, and error handling.

## Metadata
- **Complexity**: High
- **Labels**: LLVM, Codegen, Compile, MVP
- **Required Skills**: LLVM fundamentals, Rust FFI/toolchain integration, compiler testing
