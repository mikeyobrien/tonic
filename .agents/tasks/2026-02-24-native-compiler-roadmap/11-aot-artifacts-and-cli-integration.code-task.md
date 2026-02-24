---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Integrate AOT Native Artifact Pipeline into CLI (`compile`/`run`)

## Description
Add first-class CLI support for compiling to and executing native artifacts, with deterministic artifact layout, metadata, and fallback behavior.

## Background
LLVM codegen is not user-facing until CLI flow can produce and run native outputs predictably (similar operational experience to Rust/Go binaries).

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/main.rs`
- `tests/cli_contract_compile.rs`
- `tests/cli_contract_run_command.rs`
- `research/track-4-cli-tui-execution-model.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Extend `tonic compile` for backend/emit options (`llvm-ir`, object, executable).
2. Define artifact manifest metadata (backend, target triple, version, hash inputs).
3. Add native execution path (`tonic run` on native artifact) with contract-compatible output.
4. Preserve deterministic exit codes and diagnostics behavior.
5. Ensure cache invalidation keys account for backend and target-specific inputs.

## Dependencies
- LLVM backend support (tasks 05–09).
- Runtime ABI/library outputs and link strategy.
- Existing CLI contract tests and cache framework.

## Implementation Approach
1. Extend compile command parser and artifact writer.
2. Add native artifact loader/runner with clear fallback and error handling.
3. Update/expand CLI contract tests for new flags and artifact modes.

## Acceptance Criteria

1. **CLI Can Emit Native Artifacts Deterministically**
   - Given compile invocations with native backend flags
   - When compilation succeeds
   - Then artifacts and metadata are created in deterministic locations/formats.

2. **Native Artifact Execution Works via CLI**
   - Given valid native artifacts
   - When invoked with `tonic run`
   - Then execution output and exit contracts match interpreter expectations.

3. **Cache and Invalidations Are Correct**
   - Given source/backend/target changes
   - When recompiling
   - Then stale native artifacts are invalidated and rebuilt deterministically.

4. **Unit/Integration Tests Cover Native CLI Flows**
   - Given compile/run CLI contracts
   - When running `cargo test`
   - Then tests cover native emit modes, artifact loading, and diagnostics.

## Metadata
- **Complexity**: Medium
- **Labels**: CLI, AOT, Artifacts, Tooling
- **Required Skills**: CLI design, artifact management, deterministic build workflows
