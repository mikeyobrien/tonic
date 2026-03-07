# App-authoring gap catalog

This document tracks application-authoring/runtime gaps surfaced by
`/home/mobrienv/projects/tonic-sitegen-stress` and then re-checked in the main
`tonic` repo.

The rule for this catalog is simple: do not trust the stress repo by default.
Each item below is marked by what was reproduced in `tonic` itself, what landed
in `tonic`, and what still remains a product limitation.

## Evidence sources

### tonic-sitegen-stress

Primary external evidence referenced for this pass:

- `src/sitegen_fs.tn`
- `src/sitegen_string.tn`
- `src/sitegen_string_probe.tn`
- `src/sitegen_text_ingestion_probe.tn`
- `src/sitegen_text_shape_probe.tn`
- `src/sitegen_bitstring_pattern_probe.tn`
- `src/sitegen_frontmatter_byte_probe.tn`
- `src/sitegen_output_emission_probe.tn`
- `src/sitegen_system_run_repeat_probe.tn`
- `src/sitegen_static_copy_probe.tn`
- `test/verify/fs_discovery_smoke.sh`
- `test/verify/string_probe_smoke.sh`
- `test/verify/text_ingestion_probe.sh`
- `test/verify/text_shape_probe.sh`
- `test/verify/bitstring_pattern_probe.sh`
- `test/verify/frontmatter_byte_probe.sh`
- `test/verify/output_emission_probe.sh`
- `test/verify/system_run_repeat_probe.sh`
- `test/verify/system_run_repeat_fresh_fixture.sh`
- `test/verify/static_copy_probe.sh`

### tonic repo surfaces checked

- `src/manifest.rs`
- `src/interop.rs`
- `src/interop/system.rs`
- `src/interop/string_mod.rs`
- `src/interop/path_mod.rs`
- `src/c_backend/dispatcher.rs`
- `src/c_backend/funcs.rs`
- `src/c_backend/ops.rs`
- `src/c_backend/stubs.rs`
- `src/c_backend/terminator.rs`
- `docs/system-stdlib.md`
- `docs/runtime-abi.md`
- `tests/run_lazy_stdlib_loading_smoke.rs`
- `tests/system_stdlib_http_input_smoke.rs`
- `tests/runtime_llvm_string_stdlib_smoke.rs`
- `tests/runtime_llvm_system_run_repeat.rs`
- `tests/runtime_llvm_system_stdlib_smoke.rs`
- `tests/runtime_llvm_closures_bindings_interop.rs`
- `tests/runtime_llvm_bindings_call_scope.rs`
- `tests/differential_backends.rs`

## Current capability status

Read this matrix alongside [core-stdlib-profile.md](core-stdlib-profile.md).
That profile defines the current support labels; this catalog records the audit
trail behind them.

This matrix reflects the current repo state after the confirmed fixes from this
loop landed.

| Surface | Profile label | Advertised | Interpreter | Native compiled runtime | Verification evidence | Notes |
|---|---|---|---|---|---|---|
| `String.*` frontmatter helper set (`split`, `trim`, `trim_leading`, `trim_trailing`, `starts_with`, `ends_with`, `contains`, `slice`, `to_integer`) | Core-supported | Yes | Supported | Supported | `tests/run_lazy_stdlib_loading_smoke.rs::run_trace_lazy_loads_string_stdlib_module_when_referenced`; `tests/runtime_llvm_string_stdlib_smoke.rs` | Fixed by making injected `String` stdlib parseable, wiring `string_mod` into the interpreter host registry, and adding native `str_*` dispatch for the helper set app parsing currently needs. |
| `Path.join/2` (`Path.*` lazy stdlib) | Available but secondary | Yes | Supported | Supported | `tests/run_lazy_stdlib_loading_smoke.rs::run_trace_lazy_loads_path_stdlib_module_when_referenced`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_path_stdlib_join` | Fixed by wiring `path_mod` into the interpreter host registry and adding native `path_*` dispatch. |
| `System.read_text/1` | Core-supported | Yes | Supported | Supported | `tests/system_stdlib_http_input_smoke.rs::run_system_read_text_reads_file_content`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_system_read_text`; native error-shape tests in the same file | Fixed by adding native `sys_read_text` support with interpreter-matching validation/error prefixes. |
| `System.read_stdin/0` | Core-supported | Yes | Supported | Supported | `tests/system_stdlib_http_input_smoke.rs::run_system_read_stdin_reads_piped_input`; `tests/system_stdlib_http_input_smoke.rs::run_system_read_stdin_returns_empty_string_for_empty_input`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_system_read_stdin`; native arity-error test in the same file | Fixed by adding native `sys_read_stdin` support with interpreter-matching arity validation and full buffered reads. |
| Repeated successful `System.run/1` calls retaining `output` in matched maps | Core-supported | Yes | Supported | Supported | `tests/runtime_llvm_system_run_repeat.rs::compiled_runtime_retains_output_across_repeated_system_run_calls`; targeted guardrails in `tests/runtime_llvm_closures_bindings_interop.rs` and `tests/differential_backends.rs` | Fixed by restoring native pattern-binding snapshots at compiled scope boundaries so a prior `%{output: ...}` match cannot leak into later `System.run/1` result handling. |
| Stress-repo text ingestion probes (`System.read_text/1`, `System.read_stdin/0`, single captured `System.run/1`) | Confirms current core profile | Yes through the underlying stdlib contract | Supported | Supported | `test/verify/text_ingestion_probe.sh`; tonic stdlib tests above | No longer an honest blocker after the landed `System.read_text/1`, `System.read_stdin/0`, and repeated `System.run/1` fixes. |
| Stress-repo output emission / static copy probes | Confirms current core profile | Yes through `System.ensure_dir/1`, `System.write_text/2`, and `System.run/1` | Supported | Supported | `test/verify/output_emission_probe.sh`; `test/verify/static_copy_probe.sh` | No longer an upstream blocker. Static copy still leans on a shell `find` workaround for discovery, but the write/copy pipeline now succeeds in both modes. |
| Directory listing / tree walking | Core-supported | Yes — `System.list_files_recursive/1` and `System.remove_tree/1` | Supported | Supported | `tests/system_stdlib_http_input_smoke.rs`; `tests/runtime_llvm_system_stdlib_smoke.rs` | First-class stdlib contract now exists. Symlinks are skipped by both runtimes (lstat semantics). Shell workaround in the stress harness is superseded but left in place. |
| Runtime text shape for parser-style list/bitstring matching | Documented divergence | No parser-ready byte/list contract for runtime text | Binary-only | Binary-only | `tests/runtime_text_parser_contract.rs`; `test/verify/text_shape_probe.sh`; `test/verify/bitstring_pattern_probe.sh` | The current contract is now explicit: runtime text remains `is_binary: true` / `is_list: false`, so list-prefix and bitstring-byte-pattern parsers do not see parser-ready bytes today. See [text-binary-parser-contract.md](text-binary-parser-contract.md). |
| Byte-oriented frontmatter parsing ergonomics | Documented divergence | No polished runtime-text byte-parser contract | Research-grade | Research-grade | `tests/runtime_text_parser_contract.rs`; `test/verify/frontmatter_byte_probe.sh`; `tests/runtime_llvm_bindings_call_scope.rs` | Runtime text still does not enter the byte-list parser shape. The earlier compiled literal-byte abort was fixed repo-locally by isolating caller pattern bindings across named function calls, but that does not change the broader documented text/binary/parser contract gap. |

## Historical confirmed gaps fixed in this loop

### 1. `String.*` stdlib was broken before it reached runtime dispatch

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in interpreter and native compiled runtime for the frontmatter helper set covered by regressions

#### Original reproduction

A minimal project calling `String.split("a,b", ",")` originally failed during
lazy stdlib injection before any host call executed.

Original observed error:

- `error: expected (, found INT(40)`
- location pointed into injected `String` stdlib source at
  `def starts_with?(str, prefix) do`

#### Root cause confirmed in tonic

- `src/manifest.rs` advertised and lazy-loaded the `String` stdlib module.
- That embedded module used `starts_with?`, `ends_with?`, and `contains?`
  helper names that were not accepted by the current parser in this path.
- `src/interop/string_mod.rs` existed, but `src/interop.rs` did not register it
  in the active interpreter host registry.

#### Landed fix

- `src/manifest.rs` now uses parseable wrapper names for the injected `String`
  stdlib source.
- `src/interop.rs` now registers `string_mod::register_string_host_functions`.
- `src/c_backend/stubs.rs` now dispatches the native `str_*` helper set used by
  frontmatter-shaped parsing (`split`, `trim`, `trim_leading`,
  `trim_trailing`, `starts_with`, `ends_with`, `contains`, `slice`, and
  `to_integer`).
- `tests/run_lazy_stdlib_loading_smoke.rs` continues to cover interpreter lazy
  `String` stdlib load.
- `tests/runtime_llvm_string_stdlib_smoke.rs` now covers compiled literal,
  file-backed, and parse-failure cases for the same helper set.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported for the helper set covered above

---

### 2. `Path.*` stdlib was advertised but not wired into active host registries

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in interpreter and native compiled runtime

#### Original reproduction

A minimal project calling `Path.join("/tmp", "demo.txt")` originally showed:

- interpreter: `error: host error: unknown host function: path_join`
- native: `error: host error: unknown host function: path_join`

#### Root cause confirmed in tonic

- `src/manifest.rs` advertised a `Path` stdlib module with wrappers like
  `host_call(:path_join, ...)`.
- `src/interop/path_mod.rs` implemented the `path_*` handlers.
- `src/interop.rs` did not register `path_mod`.
- `src/c_backend/stubs.rs` had no native `path_*` dispatch cases.

#### Landed fix

- `src/interop.rs` now registers `path_mod::register_path_host_functions`.
- `src/c_backend/stubs.rs` now dispatches the advertised `path_*` keys used by
  the stdlib wrappers.
- `tests/run_lazy_stdlib_loading_smoke.rs` covers interpreter lazy stdlib load
  for `Path.join/2`.
- `tests/runtime_llvm_system_stdlib_smoke.rs` covers compiled `Path.join/2`.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported

---

### 3. Native `System.read_text/1` lacked parity with the interpreter

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in interpreter and native compiled runtime

#### Original reproduction

A minimal repo-local project originally showed this split:

- interpreter: printed `"hello from file"`
- native: `error: host error: unknown host function: sys_read_text`

#### Root cause confirmed in tonic

- `docs/system-stdlib.md` documented `System.read_text/1` as supported.
- `src/manifest.rs` injected the `System.read_text/1` wrapper.
- `src/interop/system.rs` already registered `sys_read_text` for interpreter
  execution.
- `src/c_backend/stubs.rs` lacked native `sys_read_text` dispatch.

#### Landed fix

- `src/c_backend/stubs.rs` now implements native `sys_read_text` dispatch.
- Native validation and I/O failure shapes now match the interpreter contract.
- `tests/runtime_llvm_system_stdlib_smoke.rs` covers compiled success,
  wrong-type rejection, and missing-file error prefix behavior.
- `tests/system_stdlib_http_input_smoke.rs` continues to cover interpreter
  success.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported

---

### 4. Native `System.read_stdin/0` lacked parity with the interpreter

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in interpreter and native compiled runtime

#### Original reproduction

A minimal repo-local project originally showed this split:

- interpreter: printed `"piped input"`
- native: `error: host error: unknown host function: sys_read_stdin`

#### Root cause confirmed in tonic

- `docs/system-stdlib.md` documented `System.read_stdin/0` as supported.
- `src/manifest.rs` injected the `System.read_stdin/0` wrapper.
- `src/interop/system.rs` already registered `sys_read_stdin` for interpreter
  execution.
- `src/c_backend/stubs.rs` lacked native `sys_read_stdin` dispatch.

#### Landed fix

- `src/c_backend/stubs.rs` now implements native `sys_read_stdin` dispatch.
- Native zero-arg validation now matches the interpreter contract.
- Native stdin reads now return the full buffered string.
- `tests/runtime_llvm_system_stdlib_smoke.rs` covers compiled stdin success and
  deterministic arity rejection.
- `tests/system_stdlib_http_input_smoke.rs` continues to cover interpreter
  piped-input and empty-input behavior.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported

---

### 5. Repeated native `System.run/1` calls leaked prior pattern bindings

**Original status:** confirmed in `tonic`  
**Priority:** P0  
**Current state:** fixed in native compiled runtime; interpreter was never affected

#### Original reproduction

A minimal repo-local fixture that called `System.run/1` twice and summarized each
result with repeated map-pattern clauses originally showed this split:

- interpreter: both calls printed `exit+output ... output=<captured text>`
- native compiled runtime: the second call fell through to
  `exit-only exit_code=0`

The tonic-local regression now lives in
`tests/runtime_llvm_system_run_repeat.rs` and matches the original
`tonic-sitegen-stress` probe behavior.

#### Root cause confirmed in tonic

This was not a `sys_run` host-call data-loss bug. The compiled runtime preserved
both command outputs until later pattern matching reused bindings across calls.

Confirmed native cause:

- compiled pattern bindings lived in process-global `tn_bindings` storage
- successful matches did not restore prior binding snapshots when compiled
  helper/function/dispatcher scopes returned
- a prior successful `%{exit_code: code, output: output}` match left `output`
  live for later matches, so the next summarize call no longer behaved like a
  fresh scope

#### Landed fix

- `src/c_backend/funcs.rs`
- `src/c_backend/terminator.rs`
- `src/c_backend/dispatcher.rs`
- `src/c_backend/stubs.rs`

now snapshot `tn_bindings` on entry to the relevant compiled scopes and restore
those snapshots before returning so branch-local bindings do not leak across
repeated calls.

Regression evidence now includes:

- `tests/runtime_llvm_system_run_repeat.rs` for the direct repeated
  `System.run/1` repro
- `tests/runtime_llvm_closures_bindings_interop.rs` for nearby closure/binding
  behavior
- `tests/differential_backends.rs::parity_catalog_subset_matches_between_interpreter_and_native`
  for targeted backend parity coverage

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported, including repeated successful
  `System.run/1` capture within the same process

---

### 6. Compiled named function calls leaked caller pattern bindings

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in native compiled runtime; interpreter was never affected

#### Original reproduction

A smaller tonic-local fixture than the stress probe reproduced the same native
abort class inside a recursive literal-byte helper:

- interpreter: returned `{:ok, [35]}`
- native compiled runtime: aborted on
  `tn_runtime_error_no_matching_clause`

The regression now lives in `tests/runtime_llvm_bindings_call_scope.rs`.

#### Root cause confirmed in tonic

This was another native binding-lifetime bug. Named function calls inherited the
caller's active `tn_bindings` entries, so callee clauses that reused names like
`rest` could not bind fresh values and incorrectly fell through to the no-clause
stub.

#### Landed fix

- `src/c_backend/ops.rs` now snapshots caller bindings before a named function
  call, clears the live table for the callee, and restores the caller snapshot
  after return.
- `tests/runtime_llvm_bindings_call_scope.rs` locks the recursive literal-byte
  helper regression so compiled native matches the interpreter.
- Nearby binding guardrails from `tests/runtime_llvm_system_run_repeat.rs`,
  `tests/runtime_llvm_closures_bindings_interop.rs`, and
  `tests/differential_backends.rs` remained green after the fix.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported for recursive literal-byte helper calls
  without leaking caller bindings into the callee

---

### 7. `System.list_files_recursive/1` and `System.remove_tree/1` — symlink parity and edge-case hardening

**Original status:** parity bug confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed in interpreter and native compiled runtime

#### Original reproduction

A fixture tree containing a real file, a symlinked file, and a symlinked directory produced divergent output:

- interpreter returned `["real/nested.txt", "root.txt"]` (symlinks silently skipped via `lstat`)
- native compiled runtime returned `["linkdir/nested.txt", "linkfile.txt", "real/nested.txt", "root.txt"]` (symlinks followed via `stat`)

The mismatch was a real parity bug for a new public filesystem traversal API.

Documentation was also inconsistent: `docs/system-stdlib.md` and `docs/runtime-abi.md` advertised the APIs while `docs/app-authoring-gaps.md` still said directory listing had no first-class stdlib contract.

#### Root cause confirmed in tonic

- Interpreter `collect_relative_files_recursive` called `entry.file_type()` which uses `lstat` semantics, so `is_file()` and `is_dir()` return false for symlinks — symlinks were silently skipped.
- Native C backend `tn_collect_relative_files_recursive` called `stat()` which follows symlinks, so `S_ISDIR` and `S_ISREG` returned true for the symlink target's type — symlinks were followed.
- `tn_remove_path_recursive` already used `lstat` correctly; `remove_tree` behavior was already consistent.

#### Landed fix

- `src/c_backend/stubs.rs`: changed `stat(child_path, &child_stat)` to `lstat(child_path, &child_stat)` in `tn_collect_relative_files_recursive` so the native traversal uses the same file-type policy as the interpreter.
- `src/c_backend/stubs.rs`: added empty-path guard for `sys_list_files_recursive` in the native dispatch, matching the interpreter and the existing `sys_remove_tree` guard.
- `src/interop/system.rs`: added empty-path guard to `host_sys_list_files_recursive`.
- `tests/system_stdlib_http_input_smoke.rs`: added symlink-parity, missing-path, type-error, empty-path, and remove-tree-symlink regressions for the interpreter.
- `tests/runtime_llvm_system_stdlib_smoke.rs`: added the same set of regressions for the native compiled runtime.
- `docs/system-stdlib.md`, `docs/runtime-abi.md`, `docs/app-authoring-gaps.md`: reconciled so all three documents agree on the API surface and symlink semantics.

#### Chosen semantics

**`System.list_files_recursive/1`:**
- Symlinks are **skipped** (not followed) in both interpreter and native runtime.
- Only real (non-symlink) regular files appear in results.
- Missing path raises; empty path raises; non-string argument raises.

**`System.remove_tree/1`:**
- Uses `lstat` in both runtimes.
- Symlinked files and symlinked directories are removed as symlinks (target not affected).
- Missing path returns `false` (idempotent); empty path raises; non-string argument raises.

#### Current support statement

- **Interpreter:** supported with documented symlink semantics
- **Native compiled runtime:** supported with matching symlink semantics

---

## Remaining app-authoring limitations and reclassifications

These are still worth tracking, but they are not the same class of issue as the
confirmed runtime/stdlib mismatches above. They sit outside the currently
core-supported profile and mostly reduce to one unresolved product question:
the text/binary/parser contract described in [core-stdlib-profile.md](core-stdlib-profile.md).

### 7. Text ingestion probes are no longer honest blockers

**Status:** reclassified to resolved upstream surface  
**Priority:** P2  
**Current state:** verified in this loop

#### Stress-repo signal

`src/sitegen_text_ingestion_probe.tn` and `test/verify/text_ingestion_probe.sh`
now pass in both interpreter and native compiled execution against the current
`tonic` repo.

That means the earlier app-authoring blocker here was the already-fixed tonic
surface area:

- native `System.read_text/1`
- native `System.read_stdin/0`
- repeated native `System.run/1` output retention

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** supported
- **Remaining blocker classification:** none for this probe after the landed
  tonic fixes

### 8. Output emission and static copy are no longer upstream blockers

**Status:** reclassified to resolved upstream surface  
**Priority:** P2  
**Current state:** verified in this loop

#### Stress-repo signal

Both of these stress-repo checks now pass against current tonic:

- `src/sitegen_output_emission_probe.tn` + `test/verify/output_emission_probe.sh`
- `src/sitegen_static_copy_probe.tn` + `test/verify/static_copy_probe.sh`

This matters because they exercise the practical app-authoring path that was
blocked before:

- repeated shell-backed capture via `System.run/1`
- filesystem writes via `System.ensure_dir/1` and `System.write_text/2`
- overwrite/re-run behavior in both interpreter and native execution

#### Current support statement

- **Interpreter:** supported for the probe workflows above
- **Native compiled runtime:** supported for the probe workflows above
- **Remaining blocker classification:** none for these probes, though static
  discovery still relies on the separate shell/workaround-only traversal story

### 9. Filesystem traversal primitives are still weak for app authoring

**Status:** capability limitation observed, plus one local stress-harness artifact  
**Priority:** P2  
**Current state:** limitation unchanged; harness expectation drift noted in this loop

#### Stress-repo signal

`src/sitegen_fs.tn` and `test/verify/fs_discovery_smoke.sh` still discover
files by combining:

- `System.path_exists/1`
- `System.run/1` with a shell `find ... | sort`

That still shows there is no first-class directory listing or tree-walk API.

On re-run in this loop, the current stress harness also failed for a narrower
reason that is **not** an upstream tonic blocker: it still expects the static
fixture to produce only `static/style.css`, but the fixture now also contains
`static/docs/guide.css`, so the shell discovery output is longer than the
hard-coded expectation.

#### Current support statement

- **Interpreter:** shell/workaround path available through `System.run/1`
- **Native compiled runtime:** shell/workaround path available through
  `System.run/1`
- **Documented first-class filesystem traversal API:** none today
- **Current `fs_discovery_smoke.sh` failure classification:** local
  stress-repo artifact, not a broken tonic contract

### 10. Runtime text shape is still binary, not parser-ready bytes

**Status:** capability limitation observed, not a broken advertised contract  
**Priority:** P2  
**Current state:** unchanged in this loop

#### Stress-repo signal

`src/sitegen_text_shape_probe.tn`, `src/sitegen_bitstring_pattern_probe.tn`,
`test/verify/text_shape_probe.sh`, and
`test/verify/bitstring_pattern_probe.sh` all succeed in both interpreter and
native execution, but they succeed by reporting the same limitation:

- runtime text values report `is_binary: true`
- runtime text values report `is_list: false`
- list-prefix matching like `[43, 43, 43, 10 | _rest]` does not treat runtime
  text as a byte list
- bitstring byte binding like `<< a, b, c, d >>` does not match runtime text
  either

#### Current support statement

- **Interpreter:** binary text available, but not parser-ready byte/list shape
- **Native compiled runtime:** binary text available, but not parser-ready
  byte/list shape
- **Documented current contract:** [text-binary-parser-contract.md](text-binary-parser-contract.md)
- **Supported parser-ish path today:** workload-backed `String` helpers on
  runtime text in project mode

### 11. Byte-oriented frontmatter parsing is still research-grade

**Status:** capability limitation, plus one native call-scope bug fixed during review  
**Priority:** P2  
**Current state:** limitation unchanged; compiled literal-byte fallback no longer aborts repo-locally

#### Stress-repo signal

`src/sitegen_frontmatter_byte_probe.tn` and
`test/verify/frontmatter_byte_probe.sh` still explore frontmatter parsing
through explicit byte-list processing.

Reclassification after the repo-local follow-up:

- interpreter literal byte-list sample still succeeds
- runtime text in both modes still fails with `reason: missing opening fence`
  because the value shape is binary text, not a byte list
- the earlier compiled literal-byte abort was a separate native backend bug,
  not a fundamental frontmatter-parser result-shape issue

The native abort is now reproduced and fixed in `tonic` with
`tests/runtime_llvm_bindings_call_scope.rs`. The root cause was caller
pattern-binding leakage across named function calls in the C backend, which let
callee matches inherit active caller bindings and fall through to
`tn_runtime_error_no_matching_clause`. `src/c_backend/ops.rs` now snapshots,
clears, and restores bindings around named function calls so the compiled
literal-byte helper path behaves like a fresh call scope.

That fix matters, but it does **not** change the broader app-authoring
conclusion: byte-list frontmatter parsing is still not a viable polished
contract for runtime text today because text values still arrive as binary text
rather than parser-ready byte lists.

#### Current support statement

- **Interpreter:** explicit literal byte-list experiments can work, but runtime
  text still does not arrive in that shape
- **Native compiled runtime:** explicit literal byte-list experiments no longer
  abort from caller-binding leakage, but runtime text still does not arrive in
  that shape
- **Documented current contract:** [text-binary-parser-contract.md](text-binary-parser-contract.md)
- **Polished runtime-text byte parser path:** none today

## Summary

This pass leaves the profile boundary cleaner: workload-backed `String` and `System` behavior is now the honest core-supported story, `Path` is usable but secondary, and the remaining major limitation is the now-documented text/binary/parser contract rather than missing breadth.

Confirmed, repo-local fixes landed in this loop:

1. `String.*` lazy stdlib loading now works in the interpreter-backed path.
2. `Path.*` host registration now works in interpreter and native execution.
3. Native `System.read_text/1` parity now matches the interpreter contract.
4. Native `System.read_stdin/0` parity now matches the interpreter contract.
5. Repeated native `System.run/1` calls now retain `output` correctly across
   repeated compiled matches in the same process.
6. Compiled named function calls now isolate caller pattern bindings so
   literal-byte helper recursion no longer aborts with
   `tn_runtime_error_no_matching_clause`.
7. `System.list_files_recursive/1` and `System.remove_tree/1` symlink parity:
   native C backend now uses `lstat` instead of `stat` for traversal, matching
   the interpreter's skip-symlinks behavior. Edge-case regressions added for
   symlinks, missing paths, type errors, and empty paths in both runtimes.
   Docs reconciled across `system-stdlib.md`, `runtime-abi.md`, and
   `app-authoring-gaps.md`.

Still-real application-authoring limitations that remain outside this fix loop:

- runtime text is still binary rather than parser-ready byte/list data for
  frontmatter-style parsers
- byte-oriented text/frontmatter processing is still awkward for real apps even
  after the repo-local native caller-binding fix, because the product-level
  text-shape limitation remains
