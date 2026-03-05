# LLVM Backend Status

## Status: Experimental

The LLVM backend (`src/llvm_backend/`) is **experimental**. It is not suitable
for production builds. Use the C backend for all production-grade compilation.

## Decision rationale

Tonic has two native backends:

| Backend | LOC | State |
|---------|-----|-------|
| C backend (`src/c_backend/`) | ~5,693 | Primary — full construct coverage, tracing GC, portable |
| LLVM backend (`src/llvm_backend/`) | ~2,471 | Experimental — partial subset, see limitations below |

Maintaining both backends at feature parity is an ongoing maintenance tax.
The decision (recorded 2026-03-04) is to demote LLVM to experimental status:
keep the code, run the parity checks informally, but stop treating parity
failures as release blockers.

## Known limitations

- Covers only a subset of MIR constructs; unsupported instructions surface as
  `LlvmBackendError::unsupported_instruction`.
- Many constructs are lowered via the `Legacy` IR wrapper, which bypasses
  native LLVM type-safe codegen.
- Hard-coded target triple: `x86_64-unknown-linux-gnu`. No cross-compilation
  support.
- LLVM compatibility pinned to version `18.1.8`; no runtime LLVM linkage —
  the backend emits `.ll` text IR only (compiled via `llc`/`clang` externally).

## Runtime warning

When the LLVM backend is exercised via `tonic compile`, the CLI prints:

```
warning: LLVM backend is experimental. Use C backend for production builds.
```

## Parity enforcement

`scripts/llvm-catalog-parity-enforce.sh` runs parity checks in
**informational mode** by default: failures are reported but exit 0.

To restore blocking behaviour (e.g. for targeted experiments):

```bash
TONIC_LLVM_PARITY_ENFORCE=1 ./scripts/llvm-catalog-parity-enforce.sh
```

## Re-promoting LLVM

To restore LLVM to primary status, the following work is needed:

1. Remove all `Legacy` IR wrappers and replace with native LLVM type-safe
   lowering for every MIR instruction.
2. Add cross-compilation support (configurable target triple).
3. Achieve full parity with the C backend across the parity fixture catalog.
4. Re-enable `--enforce` in `scripts/llvm-catalog-parity-enforce.sh` by
   default (or remove the env-var gate).
5. Update this document and `README.md` to reflect promoted status.
