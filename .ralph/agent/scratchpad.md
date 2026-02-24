# Scratchpad

## 2026-02-24T22:00:13Z — Planner bootstrap for LLVM catalog parity
Received build.start objective to execute `.agents/tasks/2026-02-24-llvm-catalog-parity/` task sequence 01→09.

Validation:
- Target directory exists.
- Contains ordered `.code-task.md` backlog files 01..09.
- No `tasks/` subdirectory (so this is a direct task-bundle directory, not a strict PDD layout).

Planner handoff intent:
- Treat this as implementation context ready for Builder.
- Queue starts at task 01 (`01-catalog-parity-harness.code-task.md`) and must proceed in order.
- Keep objective constraints attached (deterministic parity reports, fmt/clippy/tests/parity checks, CI gate at 100%, avoid unrelated dirty files).

Confidence: 95/100 (path and task queue are explicit and verified).

## 2026-02-24T22:14:40Z — Builder completed Task 01 parity harness
Implemented one-task scope: added `src/bin/llvm_catalog_parity.rs` and `tests/llvm_catalog_parity_harness.rs` to establish deterministic LLVM catalog parity reporting from `examples/parity/catalog.toml`.

What shipped:
- Harness command that executes `tonic compile <fixture> --backend llvm` for every active catalog entry.
- Direct compiled-artifact runtime execution (no `tonic run` wrapper) when compile succeeds.
- Fixture-level mismatch reasons for compile/runtime expectation drift.
- Report outputs in both JSON and Markdown, including grouped top failure causes.
- `--enforce` mode that exits non-zero when any mismatch remains.

Verification run this iteration:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --test llvm_catalog_parity_harness`
- Non-enforce parity command generated reports successfully.
- Enforce parity command failed as expected with current gaps (`compile 46/64`, `runtime 8/44`, mismatches present).

Task file updated to completed (`01-catalog-parity-harness.code-task.md`). Confidence: 96/100.

## 2026-02-24T22:14:43Z — Builder completed Task 02 literal/call compile blockers
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/02-compile-lowering-literals-and-calls.code-task.md`.

What shipped:
- LLVM lowering now handles `ConstString` + `ConstFloat` by emitting deterministic runtime helper calls (`tn_runtime_const_string`, `tn_runtime_const_float`) instead of bailing with unsupported-instruction diagnostics.
- LLVM lowering now handles unary MIR ops `to_string`, `not`, and `bang` via runtime helper calls so interpolation/literal fixture compiles no longer fail at unary op lowering.
- C backend runtime stubs now include the unary helper symbols above.
- Fixed C stub signature mismatch for `tn_runtime_map_access` (2-arg helper), removing gcc arity errors on map access fixtures.
- Added/updated regression tests:
  - `tests/compile_llvm_literals_calls_parity.rs` (covers all Task 02 fixture targets under `compile --backend llvm`)
  - `src/llvm_backend/tests.rs` now validates const string/float lowering helper emission and deterministic unsupported legacy diagnostic.
  - `tests/compile_llvm_backend_mvp.rs` deterministic unsupported-op test now anchors on unsupported `legacy` lowering (for-loop case) instead of `const_string`.

Verification run this iteration:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --test compile_llvm_backend_mvp`
- `cargo test --test compile_llvm_literals_calls_parity`
- `cargo test --test compile_aot_artifacts_cli`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-catalog-parity-task02.json --report-md /tmp/llvm-catalog-parity-task02.md`

Parity delta:
- Compile parity improved from `46/64` to `57/64`.
- Runtime parity unchanged at `8/55` (expected for this compile-focused task).

Task file updated to completed (`02-compile-lowering-literals-and-calls.code-task.md`). Confidence: 95/100.

## 2026-02-24T22:20:32Z — Builder completed Task 03 CFG/PHI inference compile blockers
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/03-compile-cfg-phi-and-map-access-fixes.code-task.md`.

What shipped:
- Added RED regression coverage in `tests/compile_llvm_backend_control_flow_calls.rs` for the task fixtures:
  - `examples/parity/02-operators/logical_keywords.tn`
  - `examples/parity/02-operators/logical_short_circuit.tn`
  - `examples/parity/06-control-flow/if_unless.tn`
- Reworked LLVM block-arg inference (`src/llvm_backend/codegen.rs`) to handle merge values not immediately consumed in the merge block:
  - infer unresolved value IDs globally (referenced but never instruction-defined),
  - prefer block-local external-use candidates,
  - deterministically fall back to unresolved IDs with assignment tracking to avoid collisions across nested merges.
- Mirrored the same inference strategy in C backend phi-slot inference (`src/c_backend/funcs.rs`) so LLVM compile path (which also lowers C sidecar) keeps deterministic merge-slot mapping.

Verification run this iteration:
- `cargo test --test compile_llvm_backend_control_flow_calls` (RED then GREEN)
- `cargo test --test compile_aot_artifacts_cli`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-catalog-parity-task03.json --report-md /tmp/llvm-catalog-parity-task03.md`

Parity delta:
- Compile parity improved from `57/64` to `60/64`.
- Runtime parity unchanged at `8/58` (compile-focused task).

Decision note:
- Logged DEC-062 (confidence 74) for unresolved-ID fallback strategy vs larger MIR schema refactor.

## 2026-02-24T22:26:01Z — Builder completed Task 04 for-legacy compile parity
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/04-compile-for-legacy-op-parity.code-task.md`.

What shipped:
- Added RED/GREEN regression coverage in `tests/compile_llvm_backend_control_flow_calls.rs` for all active `for` fixtures (`for_single_generator`, `for_multi_generator`, `for_into`, `for_into_runtime_fail`) and deterministic `for_reduce_fail` parser contract.
- Updated LLVM legacy lowering (`src/llvm_backend/codegen.rs`) to bridge `IrOp::For` through deterministic runtime helper calls (`tn_runtime_for`) instead of rejecting as unsupported legacy.
- Updated C backend legacy lowering (`src/c_backend/ops.rs`) with the same `Try`/`For` runtime-helper bridge so LLVM compile path remains coherent with generated C sidecar.
- Extended native stub declarations (`src/c_backend/stubs.rs`) and LLVM declarations to include `tn_runtime_for(i64)`.
- Updated deterministic legacy tests (`src/llvm_backend/tests.rs`, `tests/compile_llvm_backend_mvp.rs`) so unsupported-legacy coverage remains explicit while `for` compile support is now expected.

Verification run this iteration:
- `cargo test --test compile_llvm_backend_control_flow_calls` (RED then GREEN)
- `cargo test --test compile_llvm_backend_mvp`
- `cargo test lower_mir_subset_lowers_for_legacy_instruction_to_runtime_helper`
- `cargo test lower_mir_subset_rejects_unsupported_legacy_instruction`
- `cargo test --test compile_aot_artifacts_cli`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-catalog-parity-task04.json --report-md /tmp/llvm-catalog-parity-task04.md`

Parity delta:
- Compile parity improved from `60/64` to `64/64`.
- Runtime parity now `8/62` (expected drop in percentage because four newly compiling fixtures are now counted in runtime stage and still wait on runtime helper implementation tasks).

Task file updated to completed (`04-compile-for-legacy-op-parity.code-task.md`). Confidence: 94/100.

## 2026-02-24T22:37:58Z — Builder completed Task 05 runtime core values/collections parity
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/05-runtime-core-values-and-collections.code-task.md`.

What shipped:
- Added a native C runtime object model in `src/c_backend/stubs.rs` for core value types used by active fixtures (bool/nil, atom, string, float, tuple, list, map, keyword, range), plus deterministic renderer `tn_runtime_println` for direct executable output parity.
- Implemented runtime helper paths (no longer stub-abort) for:
  - `tn_runtime_const_atom`
  - `tn_runtime_const_string`
  - `tn_runtime_const_float`
  - `tn_runtime_make_tuple`
  - `tn_runtime_make_list` (varargs)
  - `tn_runtime_range`
  - `tn_runtime_in`
  - map/keyword constructors + mutation/access (`map_empty`, `make_map`, `map_put`, `map_update`, `map_access`, `make_keyword`, `keyword_append`)
- Updated C codegen (`src/c_backend/ops.rs`) to emit runtime literal helpers for atom/string/float and boxed-bool literals, plus escaped string literal emission for generated C.
- Updated C terminator lowering (`src/c_backend/terminator.rs`) to branch via runtime truthiness helper and use runtime equality for bool/nil pattern checks.
- Updated C main entrypoint (`src/c_backend/decls.rs`) to print rendered runtime values instead of raw int64 numbers.
- Added runtime regression coverage: `tests/runtime_llvm_core_values_collections.rs` asserting compile+direct-executable stdout parity across core literal/collection fixtures.

Verification run this iteration:
- `cargo test --test runtime_llvm_core_values_collections` (RED then GREEN)
- `cargo test --test compile_aot_artifacts_cli`
- `cargo test --test compile_llvm_literals_calls_parity`
- `cargo test --test compile_llvm_backend_control_flow_calls`
- `cargo fmt --all -- --check` (after `cargo fmt --all`)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-parity-task05.json --report-md /tmp/llvm-parity-task05.md`

Parity delta:
- Compile parity remains `64/64`.
- Runtime parity improved from `8/62` to `31/62`.
- No remaining runtime stderr failures for Task 05 helper bucket (`const_atom`, `make_tuple`, `make_list`, `range`, map/keyword constructor/access/update paths).

Task file updated to completed (`05-runtime-core-values-and-collections.code-task.md`). Confidence: 88/100.

## 2026-02-24T23:06:12Z — Builder completed Task 06 runtime patterns/guards/control-flow parity
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/06-runtime-patterns-guards-and-control-flow.code-task.md`.

What shipped:
- Added generated native pattern runtime in `src/c_backend/runtime_patterns.rs` with:
  - deterministic hash-indexed pattern dispatch (`tn_runtime_pattern_matches`),
  - binding table snapshot/restore for backtracking-safe pattern checks,
  - runtime binding loads (`tn_runtime_load_binding`),
  - `match` operator runtime behavior (`tn_runtime_match_operator`) with bad-match fallback.
- Supported pattern semantics used by active fixtures: atom/string/int/bool/nil, bind/pin/wildcard, tuple/list (with tail support), and map entry matching with binding rollback.
- Updated `src/c_backend/stubs.rs` + `src/c_backend/mod.rs` to wire generated pattern helpers and keep non-task helpers as explicit abort stubs.
- Updated both C and LLVM terminator pattern fast-path logic so `Bind` patterns go through runtime matcher (ensures bindings are created before guard/load access).
- Added RED→GREEN runtime regression coverage in `tests/runtime_llvm_patterns_control_flow.rs` for all Task 06 fixture targets.

Verification run this iteration:
- `cargo test --test runtime_llvm_patterns_control_flow` (RED then GREEN)
- `cargo test --test compile_llvm_backend_collections_patterns`
- `cargo test --test compile_llvm_backend_control_flow_calls`
- `cargo test --test runtime_llvm_core_values_collections`
- `cargo fmt --all -- --check` (after `cargo fmt --all`)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-parity-task06.json --report-md /tmp/llvm-parity-task06.md`

Parity delta:
- Compile parity remains `64/64`.
- Runtime parity improved from `31/62` to `46/62`.
- All Task 06 target fixtures now report runtime parity match.

Task file updated to completed (`06-runtime-patterns-guards-and-control-flow.code-task.md`). Confidence: 87/100.

## 2026-02-24T22:57:52Z — Builder completed Task 07 runtime closures/bindings/interop parity
Implemented one-task scope for `.agents/tasks/2026-02-24-llvm-catalog-parity/07-runtime-closures-bindings-and-interop.code-task.md`.

What shipped:
- Added compiled-closure runtime support in `src/c_backend/stubs.rs`:
  - closure object kind (`TN_OBJ_CLOSURE`) and renderer support,
  - native `tn_runtime_make_closure` object construction,
  - native `tn_runtime_call_closure_varargs` invocation path with arity/type checks,
  - MIR-driven closure descriptor dispatch generation (`tn_runtime_call_compiled_closure`) with closure body lowering for active closure ops.
- Implemented native host interop helpers in `src/c_backend/stubs.rs`:
  - `tn_runtime_host_call_varargs` for `:identity` and `:sum_ints`,
  - deterministic unknown host/key/type/arity failures,
  - `tn_runtime_protocol_dispatch` mapping tuple->1 and map->2 with deterministic unsupported-type failure.
- Updated binding load miss behavior in `src/c_backend/runtime_patterns.rs` to deterministic runtime failure (no `tn_stub_abort("tn_runtime_load_binding")` fallback path).
- Added RED→GREEN regression coverage in `tests/runtime_llvm_closures_bindings_interop.rs` for:
  - catalog fixtures `anonymous_fn_capture_invoke`, `cond_branches`, `host_call_and_protocol_dispatch`,
  - protocol dispatch runtime behavior,
  - deterministic unsupported interop errors.

Verification run this iteration:
- `cargo test --test runtime_llvm_closures_bindings_interop` (RED then GREEN)
- `cargo test --test compile_llvm_backend_closures_captures`
- `cargo test --test compile_llvm_backend_host_interop`
- `cargo test --test runtime_llvm_patterns_control_flow`
- `cargo test --test compile_aot_artifacts_cli`
- `cargo fmt --all -- --check` (after `cargo fmt --all`)
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin llvm_catalog_parity -- --catalog examples/parity/catalog.toml --report-json /tmp/llvm-parity-task07.json --report-md /tmp/llvm-parity-task07.md`

Parity delta:
- Compile parity remains `64/64`.
- Runtime parity improved from `46/62` to `48/62`.
- Task 07 target fixtures now all match catalog output/exit in compiled mode.

Task file updated to completed (`07-runtime-closures-bindings-and-interop.code-task.md`). Confidence: 89/100.
