---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Build Native Runtime Library for Core Primitives and Semantics

## Description
Implement a native runtime support library used by generated LLVM code for arithmetic, comparisons, collections, pattern checks, and core builtins.

## Background
LLVM codegen should not inline every dynamic semantic operation early. A runtime helper library gives correctness-first behavior and incremental optimization opportunities.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `src/runtime.rs`
- `src/ir.rs`
- `src/manifest.rs`
- `research/runtime-architecture.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Implement native runtime helpers for numeric/logical operators and comparison behavior.
2. Implement helpers for list/tuple/map/keyword construction and mutation patterns needed by language semantics.
3. Implement helper APIs for `case`/pattern primitive checks used by generated code.
4. Implement deterministic error-return conventions matching current diagnostics behavior.
5. Ensure helper APIs are backend-friendly and callable from LLVM IR.

## Dependencies
- Runtime ABI from task 03.
- MIR op taxonomy from task 02.
- Existing interpreter semantics as source of truth.

## Implementation Approach
1. Add `src/native_runtime/*` modules grouped by concern (value ops, collections, pattern helpers).
2. Mirror interpreter semantics with shared assertion fixtures.
3. Create conformance tests that run helper APIs against known result snapshots.

## Acceptance Criteria

1. **Core Helper Coverage Exists**
   - Given operator and collection-heavy fixtures
   - When executed through native runtime helper tests
   - Then results match interpreter semantics.

2. **Error Semantics Match Existing Contracts**
   - Given invalid operations (type mismatch, divide by zero, invalid pattern ops)
   - When helpers are invoked
   - Then deterministic errors match current message/offset expectations where applicable.

3. **Backend Integration Interface Is Usable**
   - Given a mock codegen caller
   - When invoking helper entrypoints through ABI interfaces
   - Then calls succeed with stable signatures and no undefined behavior.

4. **Unit/Integration Tests Cover Helpers Thoroughly**
   - Given native runtime helper modules
   - When running `cargo test`
   - Then tests cover arithmetic, collections, pattern checks, and negative cases.

## Metadata
- **Complexity**: High
- **Labels**: Runtime, Primitives, Semantics, Native-Backend
- **Required Skills**: Rust runtime engineering, dynamic language semantics, API design
