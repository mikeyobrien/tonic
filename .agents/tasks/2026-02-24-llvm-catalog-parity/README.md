# LLVM Catalog Parity to 100% — Task Sequence

Goal: bring `tonic compile --backend llvm` + direct executable runtime to full parity with `examples/parity/catalog.toml`.

## Baseline (2026-02-24 snapshot)

Measured against active catalog entries:
- Active entries: **64**
- Compile expectation match (`check_exit`): **46 / 64**
- Compile expectation mismatches: **18**
- Runtime parity on expected-compile-success entries: **8 / 62**

Top gap buckets:
1. LLVM lowering compile blockers
   - `const_string` (9 fixtures)
   - `const_float` (1 fixture)
   - CFG block-arg inference (3 fixtures)
   - map access C arity/signature mismatch (1 fixture)
   - `legacy` for-op lowering (4 fixtures)
2. Native runtime helper gaps
   - `tn_runtime_const_atom` (14)
   - `tn_runtime_make_tuple` (8)
   - `tn_runtime_make_list` (5)
   - `tn_runtime_try` (4)
   - `tn_runtime_make_err` (1), `tn_runtime_make_ok` (1)
   - `tn_runtime_range` (1)
   - `tn_runtime_make_closure` (1)
   - `tn_runtime_load_binding` (1)

## Sequence

1. `01-catalog-parity-harness.code-task.md`
2. `02-compile-lowering-literals-and-calls.code-task.md`
3. `03-compile-cfg-phi-and-map-access-fixes.code-task.md`
4. `04-compile-for-legacy-op-parity.code-task.md`
5. `05-runtime-core-values-and-collections.code-task.md`
6. `06-runtime-patterns-guards-and-control-flow.code-task.md`
7. `07-runtime-closures-bindings-and-interop.code-task.md`
8. `08-runtime-results-errors-and-try.code-task.md`
9. `09-catalog-100-percent-gate-and-ci.code-task.md`

## Definition of Done

- All active catalog entries satisfy `check_exit` under `compile --backend llvm`.
- For all entries with `check_exit = 0`, direct executable output matches catalog `run_exit/stdout/stderr_contains`.
- Deterministic parity report produced in CI and enforceable.
