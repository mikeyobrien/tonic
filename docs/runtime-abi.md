# Native Runtime ABI (v1)

This document defines the stable ABI used by native codegen/runtime boundaries.

## Version

- `TONIC_RUNTIME_ABI_VERSION = 1`
- ABI version is carried by `TCallContext.abi_version`.
- Mismatched versions return `TCallStatus::InvalidAbi`.

## TValue layout

`TValue` is `#[repr(C)]` and has fixed v1 layout:

- size: 16 bytes
- alignment: 8 bytes
- fields:
  - `tag: u8` (`TValueTag`)
  - `ownership: u8` (`TOwnership`)
  - `reserved: u16` (must be `0`)
  - `payload: u64`

### Tag and payload policy

Immediate payloads:
- `Int` => two's-complement `i64` in `payload`
- `Bool` => `0` or `1`
- `Nil` => `0`

Heap payloads (refcounted handle IDs):
- `Float`, `String`, `Atom`, `List`, `Map`, `Keyword`, `Tuple2`, `ResultOk`, `ResultErr`, `Closure`, `Range`

## Memory policy

v1 uses deterministic handle-based reference counting:

- heap values are stored in an internal handle table
- `retain_tvalue` increments refcount
- `release_tvalue` decrements refcount and frees at zero
- releasing an unknown/already-freed handle returns deterministic `AbiErrorCode::InvalidHandle`
- ownership/tag misuse returns deterministic `AbiErrorCode::OwnershipViolation` or `TagHandleMismatch`

## Boundary call ABI

`TCallContext`:
- `abi_version`
- `argc`
- `argv: *const TValue`

`TCallResult`:
- `status: TCallStatus` (`Ok | Err | Panic | InvalidAbi`)
- `value: TValue` (valid for `Ok`)
- `error: TValue` (string payload for `Err/Panic/InvalidAbi`)

`invoke_runtime_boundary(...)` behavior:
- validates ABI version + frame shape
- decodes args via `tvalue_to_runtime`
- catches panics (`catch_unwind`) and returns `Panic` status instead of unwinding across boundary
- maps helper/runtime ABI failures to deterministic `Err` result

## Conversion helpers

- `runtime_to_tvalue(RuntimeValue) -> Result<TValue, AbiError>`
- `tvalue_to_runtime(TValue) -> Result<RuntimeValue, AbiError>`
- `validate_tvalue(TValue) -> Result<(), AbiError>`

These helpers are used for differential testing between interpreter values and native ABI values.
