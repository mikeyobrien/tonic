# Native Runtime Helpers (Task 04)

This runtime layer provides semantics helpers for LLVM/native codegen while preserving
current interpreter behavior.

## Modules

- `src/native_runtime/ops.rs`
  - numeric/logical/comparison helpers (`add_int`, `sub_int`, `mul_int`, `div_int`, `cmp_int`)
  - truthiness/logical helpers (`strict_not`, `truthy_bang`)
  - string/list/range helpers (`concat`, `in_operator`, `list_concat`, `list_subtract`, `range`)
- `src/native_runtime/collections.rs`
  - constructors and mutation helpers for tuple/list/map/keyword values
- `src/native_runtime/pattern.rs`
  - `match_pattern` and `select_case_branch` for `case`/match primitive checks
- `src/native_runtime/interop.rs`
  - host interop adapter helpers (`host_call`, `protocol_dispatch`) and ABI version constant
- `src/native_runtime/boundary.rs`
  - ABI-callable exported helper entrypoints

## ABI-callable helper entrypoints

Each helper is callable with the stable Task 03 ABI boundary:

```rust
extern "C" fn(TCallContext) -> TCallResult
```

Exported symbols:

- `tonic_rt_add_int`
- `tonic_rt_cmp_int_eq`
- `tonic_rt_map_put`
- `tonic_rt_host_call`
- `tonic_rt_protocol_dispatch`

All entrypoints route through `native_abi::invoke_runtime_boundary`, so they inherit:

- ABI version checking
- `TValue` decoding/validation
- panic containment
- deterministic error return (`TCallStatus::Err` + error payload)

## Deterministic error contract

Helpers return deterministic messages that mirror existing runtime semantics, including
source offset attachment when provided by caller (e.g. `"division by zero at offset 44"`,
`"badarg at offset <n>"`, map key/update contract errors).

## LLVM closure helper contract (Task 09)

LLVM lowering now reserves runtime helper symbols for closure semantics:

- `tn_runtime_make_closure(i64 descriptor_hash, i64 arity, i64 capture_count)`
- `tn_runtime_call_closure(i64 closure_value, i64 argc, ...)`

These symbols preserve deterministic compile-time contracts for anonymous function creation,
lexical capture metadata, and function-value invocation in the native backend.

## LLVM host interop helper contract (Task 10)

LLVM lowering now reserves host interop helper symbols for native backend calls:

- `tn_runtime_host_call(i64 argc, ...)`
- `tn_runtime_protocol_dispatch(i64 value)`

`host_call` preserves existing atom-key validation and deterministic host-registry error
messages through the runtime interop adapter. `protocol_dispatch` preserves tuple/map
implementation mapping behavior used by interpreter mode.

Host interop ABI policy constant:

- `TONIC_HOST_INTEROP_ABI_VERSION = 1`

## Memory management planning

For runtime memory strategy research and implementation scaffolding, see:

- `docs/memory-management-roadmap.md`
- `docs/runtime-memory-task-scaffold.md`

Task 01 observability additions:

- Generated C runtime supports opt-in memory diagnostics with
  `TONIC_MEMORY_STATS=1`.
- Diagnostics emit one deterministic line on stderr:
  `memory.stats c_runtime ...`.
- Baseline harness: `scripts/memory-baseline.sh`
- Stress fixtures: `examples/memory/*.tn`

Task 03 RC prototype additions:

- RC mode is opt-in via `TONIC_MEMORY_MODE=rc` (default remains append-only).
- Stats include `memory_mode`, `reclaims_total`, `heap_live_slots`, and
  `cycle_collection=off` (cycle caveat is explicit in RC mode).

Task 04 tracing GC prototype additions:

- Tracing mode is opt-in via `TONIC_MEMORY_MODE=trace`.
- Tracing collector is non-moving mark/sweep over boxed heap objects.
- Root traversal includes runtime root stack plus bool/nil singletons.
- Main entrypoint performs deterministic stop-the-world collection before exit
  (`tn_runtime_gc_collect`) so cyclic garbage can be reclaimed.
- Stats include `cycle_collection=mark_sweep` and `gc_collections_total` in
  trace mode.
