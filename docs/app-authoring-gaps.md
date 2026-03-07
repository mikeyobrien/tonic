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
- `src/sitegen_frontmatter_byte_probe.tn`
- `test/verify/fs_discovery_smoke.sh`
- `test/verify/string_probe_smoke.sh`
- `test/verify/text_ingestion_probe.sh`
- `test/verify/frontmatter_byte_probe.sh`

### tonic repo surfaces checked

- `src/manifest.rs`
- `src/interop.rs`
- `src/interop/system.rs`
- `src/interop/string_mod.rs`
- `src/interop/path_mod.rs`
- `src/c_backend/stubs.rs`
- `docs/system-stdlib.md`
- `docs/runtime-abi.md`
- `tests/run_lazy_stdlib_loading_smoke.rs`
- `tests/system_stdlib_http_input_smoke.rs`
- `tests/runtime_llvm_system_stdlib_smoke.rs`

## Current capability status

This matrix reflects the current repo state after the confirmed fixes from this
loop landed.

| Surface | Advertised | Interpreter | Native compiled runtime | Regression coverage | Notes |
|---|---|---|---|---|---|
| `String.split/2` (`String.*` lazy stdlib) | Yes | Supported | Not a native-specific target in this loop | `tests/run_lazy_stdlib_loading_smoke.rs::run_trace_lazy_loads_string_stdlib_module_when_referenced` | Fixed by making injected `String` stdlib parseable and wiring `string_mod` into the interpreter host registry. |
| `Path.join/2` (`Path.*` lazy stdlib) | Yes | Supported | Supported | `tests/run_lazy_stdlib_loading_smoke.rs::run_trace_lazy_loads_path_stdlib_module_when_referenced`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_path_stdlib_join` | Fixed by wiring `path_mod` into the interpreter host registry and adding native `path_*` dispatch. |
| `System.read_text/1` | Yes | Supported | Supported | `tests/system_stdlib_http_input_smoke.rs::run_system_read_text_reads_file_content`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_system_read_text`; native error-shape tests in the same file | Fixed by adding native `sys_read_text` support with interpreter-matching validation/error prefixes. |
| `System.read_stdin/0` | Yes | Supported | Supported | `tests/system_stdlib_http_input_smoke.rs::run_system_read_stdin_reads_piped_input`; `tests/system_stdlib_http_input_smoke.rs::run_system_read_stdin_returns_empty_string_for_empty_input`; `tests/runtime_llvm_system_stdlib_smoke.rs::compiled_runtime_supports_system_read_stdin`; native arity-error test in the same file | Fixed by adding native `sys_read_stdin` support with interpreter-matching arity validation and full buffered reads. |
| Directory listing / tree walking | No first-class stdlib contract | Workaround-only | Workaround-only | Stress-repo shell-based probes | Still a product limitation, not a broken advertised contract. |
| Byte-oriented frontmatter parsing ergonomics | No stable contract | Research-grade | Research-grade | Stress-repo probe only | Still a capability note, not a confirmed runtime bug from this loop. |

## Historical confirmed gaps fixed in this loop

### 1. `String.*` stdlib was broken before it reached runtime dispatch

**Original status:** confirmed in `tonic`  
**Priority:** P1  
**Current state:** fixed for interpreter-backed lazy stdlib loading

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
- `tests/run_lazy_stdlib_loading_smoke.rs` now covers lazy `String` stdlib load
  and `String.split/2` success.

#### Current support statement

- **Interpreter:** supported
- **Native compiled runtime:** not expanded beyond the loop's targeted contract;
  this slice fixed the advertised interpreter-backed lazy stdlib path

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

## Remaining app-authoring limitations

These are still worth tracking, but they are not the same class of issue as the
confirmed runtime/stdlib mismatches above.

### 5. Filesystem traversal primitives are still weak for app authoring

**Status:** capability limitation observed, not a broken advertised contract  
**Priority:** P2  
**Current state:** unchanged in this loop

#### Stress-repo signal

`src/sitegen_fs.tn` and `test/verify/fs_discovery_smoke.sh` discover files by
combining:

- `System.path_exists/1`
- `System.run/1` with a shell `find ... | sort`

That works today, including in the stress-repo harness, but it also shows there
is still no first-class directory listing or tree-walk API being used.

#### Current support statement

- **Interpreter:** shell/workaround path available through `System.run/1`
- **Native compiled runtime:** shell/workaround path available through
  `System.run/1`
- **Documented first-class filesystem traversal API:** none today

### 6. Byte-oriented frontmatter parsing is still research-grade

**Status:** suspected limitation / not yet a documented contract  
**Priority:** P2  
**Current state:** unchanged in this loop

#### Stress-repo signal

`src/sitegen_frontmatter_byte_probe.tn` and
`test/verify/frontmatter_byte_probe.sh` explore frontmatter parsing through
explicit byte-list processing.

#### Current support statement

- **Interpreter:** possible, but ergonomically rough
- **Native compiled runtime:** possible only to the extent the same language
  constructs are supported
- **Documented polished contract:** none today

## Summary

Confirmed, repo-local fixes landed in this loop:

1. `String.*` lazy stdlib loading now works in the interpreter-backed path.
2. `Path.*` host registration now works in interpreter and native execution.
3. Native `System.read_text/1` parity now matches the interpreter contract.
4. Native `System.read_stdin/0` parity now matches the interpreter contract.

Still-real application-authoring limitations that remain outside this fix loop:

- first-class filesystem traversal APIs are still thin
- byte-oriented text/frontmatter processing is still awkward for real apps
