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

All three route through `native_abi::invoke_runtime_boundary`, so they inherit:

- ABI version checking
- `TValue` decoding/validation
- panic containment
- deterministic error return (`TCallStatus::Err` + error payload)

## Deterministic error contract

Helpers return deterministic messages that mirror existing runtime semantics, including
source offset attachment when provided by caller (e.g. `"division by zero at offset 44"`,
`"badarg at offset <n>"`, map key/update contract errors).
