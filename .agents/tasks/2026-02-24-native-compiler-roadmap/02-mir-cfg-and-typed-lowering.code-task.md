---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Introduce MIR with Explicit CFG and Typed Lowering Boundaries

## Description
Design and implement a compiler-friendly MIR layer between AST/IR and backend codegen, with explicit control flow and value operations suitable for LLVM lowering.

## Background
Current `IrOp` is interpreter-oriented and stack-like. LLVM backend implementation will be fragile unless control flow, temporaries, and operation semantics are represented explicitly in a backend-neutral MIR.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/ir.rs`
- `src/parser.rs`
- `src/typing.rs`
- `research/runtime-architecture.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define MIR data model with modules/functions/basic blocks/terminators.
2. Encode explicit branch and phi-like merge semantics (or pre-SSA form with clear block arguments).
3. Define typed value categories required for lowering dynamic runtime values.
4. Implement lowering pass from existing front-end IR/AST into MIR for currently supported language features.
5. Provide MIR serialization/dump mode for debugging and tests.

## Dependencies
- Existing parser/resolver/typing front-end.
- Existing IR lowering semantics and diagnostics contracts.

## Implementation Approach
1. Create `src/mir.rs` with stable, deterministic structures.
2. Add `lower_to_mir` pass and wire compile pipeline to optionally emit MIR.
3. Add golden tests for MIR shape and control-flow determinism.

## Acceptance Criteria

1. **MIR Captures Control Flow Deterministically**
   - Given representative programs with branching and pattern dispatch
   - When lowering to MIR
   - Then generated blocks and edges are deterministic and stable across runs.

2. **Typed Lowering Boundaries Are Enforced**
   - Given front-end typed AST/IR
   - When converting to MIR
   - Then MIR values/ops reflect required runtime typing boundaries for codegen.

3. **MIR Debug Output Works**
   - Given `tonic check`/`tonic compile` with MIR dump option
   - When invoked on fixtures
   - Then MIR output is valid, deterministic, and testable.

4. **Unit/Integration Tests Cover MIR and Lowering**
   - Given MIR model and lowering pass
   - When running `cargo test`
   - Then tests cover block structure, terminators, and core language constructs.

## Metadata
- **Complexity**: High
- **Labels**: Compiler, MIR, CFG, Architecture
- **Required Skills**: Compiler IR design, Rust enums/ownership, control-flow lowering
