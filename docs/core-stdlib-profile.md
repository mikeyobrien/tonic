# Tonic Core Stdlib Profile

Status: reviewable baseline  
Last updated: 2026-03-09

This document defines the **current honest Tonic Core Stdlib** for real programs.

The goal is not to clone Elixir's full standard library. The goal is to ship a **small, believable, Elixir-shaped subset** whose exposed surface, implementation boundary, docs, and tests all agree.

## Product position

Tonic should be understood as:

- Elixir-inspired syntax and code shape
- a non-BEAM runtime
- an interpreted/native language with a host-backed optional stdlib

That means Tonic should:

- borrow Elixir module names and function shapes where practical
- avoid claiming BEAM/OTP semantics it does not implement
- prefer explicit divergence over implied parity
- advertise only surfaces that are actually supported end to end

## Execution-mode caveat

Important current limitation: optional stdlib injection still happens in **project mode**, not in plain single-file execution.

Today this means:

- `tonic run <project-dir>` and `tonic compile <project-dir>` can lazy-load optional stdlib modules from `src/manifest.rs`
- `tonic run file.tn` does **not** currently receive that optional stdlib injection

So the current stdlib contract is both **module-dependent** and **execution-mode-dependent**. That caveat remains real until single-file and project-mode behavior are intentionally unified.

## Evidence base for this profile

This baseline is grounded in:

- `src/manifest.rs`
- `src/interop.rs`
- `src/interop/system.rs`
- `src/interop/string_mod.rs`
- `src/interop/path_mod.rs`
- `src/interop/io_mod.rs`
- `src/interop/map_mod.rs`
- `src/interop/enum_mod.rs`
- `src/c_backend/stubs.rs`
- `tests/run_lazy_stdlib_loading_smoke.rs`
- `tests/runtime_llvm_map_stdlib_smoke.rs`
- `tests/runtime_llvm_io_stdlib_smoke.rs`
- `examples/apps/stdlib_showcase`

The original profile work was driven by `tonic-sitegen-stress`, but the current baseline is broader than that one workload. The support bar is now: exposed surface + interpreter behavior + native behavior + regression coverage + docs that match reality.

## Core profile rules

A stdlib surface belongs in the current Tonic Core Stdlib only if all of the following are true:

1. **The module is intentionally exposed** by `src/manifest.rs`.
2. **Interpreter behavior exists** and is intentional.
3. **Native compiled behavior exists** for every public host-backed helper that the module exposes.
4. **Regression coverage exists** for the supported behavior.
5. **Docs and examples match the real boundary**.

If any of those are missing, the surface may still be a future candidate, but it is not part of the supported core profile.

## Support labels used in this document

| Label | Meaning |
|---|---|
| **Core-supported** | Publicly exposed in project mode, intentional, covered, and aligned across interpreter/native behavior |
| **Deferred** | Intentionally not part of the current supported project-mode stdlib surface |

## Current optional stdlib surface

At the manifest-injection level, the current optional project-mode stdlib surface is exactly:

- `System`
- `String`
- `Path`
- `IO`
- `List`
- `Map`
- `Enum`

These modules lazy-load in project mode when referenced.

## Required implementation split

The current support boundary is intentionally split between pure Tonic code and host-backed primitives.

### Host-backed modules and surfaces

#### `System` — Core-supported

`System` remains host-backed. It is the practical boundary for:

- filesystem operations
- process execution
- environment access
- CLI input/output helpers
- HTTP client/server helpers
- selected crypto helpers

Representative supported surface includes:

- `System.path_exists/1`
- `System.list_files_recursive/1`
- `System.remove_tree/1`
- `System.ensure_dir/1`
- `System.read_text/1`
- `System.write_text/2`
- `System.read_stdin/0`
- `System.run/1`
- `System.argv/0`
- `System.cwd/0`
- `System.env/1`
- `System.which/1`

#### `String` — Core-supported

`String` remains host-backed because the current text contract and UTF-8-sensitive behavior are runtime concerns, not a good target for a forced pure-Tonic rewrite.

Representative supported surface includes:

- `String.split/2`
- `String.trim/1`
- `String.trim_leading/1`
- `String.trim_trailing/1`
- `String.starts_with/2`
- `String.ends_with/2`
- `String.contains/2`
- `String.slice/3`
- `String.to_integer/1`
- `String.to_charlist/1`

#### `Path` — Core-supported

`Path` remains host-backed. It is small, useful, and already parity-backed.

Current public surface:

- `Path.join/2`
- `Path.dirname/1`
- `Path.basename/1`
- `Path.extname/1`
- `Path.expand/1`
- `Path.relative_to/2`

#### `IO` — Core-supported

`IO` is public again, but it intentionally remains host-backed.

Current public surface:

- `IO.puts/1`
- `IO.inspect/1`
- `IO.gets/1`
- `IO.ansi_red/1`
- `IO.ansi_green/1`
- `IO.ansi_yellow/1`
- `IO.ansi_blue/1`
- `IO.ansi_reset/0`

Reason: stdin/stdout/stderr and ANSI behavior are runtime primitives, not library code.

#### `Map` — Core-supported, bounded host-backed surface

`Map` is public again, but only for the bounded surface Tonic can honestly support today.

Current public surface:

- `Map.keys/1`
- `Map.values/1`
- `Map.merge/2`
- `Map.drop/2`
- `Map.take/2`
- `Map.get/3`
- `Map.put/3`
- `Map.delete/2`

Current non-goals:

- `Map.has_key?/2` is still deferred until the parser/public surface can expose `?`-suffixed APIs cleanly.
- `Map.filter/2` and `Map.reject/2` are not advertised because the current public language/runtime story is not yet good enough to claim them honestly.

### Pure-Tonic modules and mixed surfaces

#### `List` — Core-supported, pure Tonic

`List` is public and implemented directly in injected Tonic source.

Current public surface:

- `List.first/1`
- `List.last/1`
- `List.wrap/1`
- `List.flatten/1`
- `List.zip/2`
- `List.unzip/1`

These are pure structural transforms and are a better fit in Tonic than in Rust host glue.

#### `Enum` — Core-supported, mixed pure/host split

`Enum` is public again, but it has an intentional split.

Pure Tonic helpers:

- `Enum.count/1`
- `Enum.sum/1`
- `Enum.reverse/1`
- `Enum.take/2`
- `Enum.drop/2`
- `Enum.chunk_every/2`
- `Enum.unique/1`
- `Enum.into/2` for list and bounded map collectables

Remaining host-backed helpers:

- `Enum.join/2`
- `Enum.sort/1`

Reason: the pure transforms are cleanly expressible in Tonic today, while `join` and `sort` still rely on runtime-side stringification/comparison behavior.

## Current status matrix

| Module/surface | Profile status | Implementation shape |
|---|---|---|
| `System` | Core-supported | Host-backed |
| `String` | Core-supported | Host-backed |
| `Path` | Core-supported | Host-backed |
| `IO` | Core-supported | Host-backed |
| `List` | Core-supported | Pure Tonic |
| `Map` | Core-supported | Bounded host-backed surface |
| `Enum` | Core-supported | Mixed pure/host split |
| `URI` / `Keyword` / `Integer` / `Float` / `Tuple` / `OptionParser` / `Regex` / `Stream` | Deferred | Not part of the current public optional stdlib surface |

## Parity policy

A stdlib function should be described as **supported** only when all of the following are true:

1. **Project-mode exposure exists** in `src/manifest.rs`.
2. **Interpreter support exists** for the intended argument and error contract.
3. **Native compiled support exists** for the same contract when the function is host-backed.
4. **Any known divergence is documented**.
5. **Regression coverage exists** in repo tests.
6. **Advertising matches reality** across docs, examples, and module lists.

A function should **not** be advertised as supported when any of these are true:

- only the interpreter works
- only the native path works
- the wrapper exists but native dispatch does not
- the module injects but the docs still describe it as deferred
- the docs describe behavior that no regression test covers

## Intentional divergence from Elixir

Tonic should be Elixir-shaped where practical, but this profile intentionally does **not** promise BEAM/OTP semantics.

Tonic does **not** claim support for:

- `GenServer`
- `Supervisor`
- `Task`
- `Agent`
- `Process`
- `Node`
- Erlang mailbox/process/distribution semantics
- implicit OTP application/runtime behavior

Tonic also still has an explicit text/runtime divergence:

- runtime text is still binary-shaped rather than a parser-ready byte/list type
- parser-style byte decomposition is not the current runtime text contract
- parser-heavy code should still lean on the supported `String.*` helpers or explicit workload-specific helpers

See [text-binary-parser-contract.md](text-binary-parser-contract.md).

## Example reference

For a small runnable example of the current project-mode surface, see:

- `examples/apps/stdlib_showcase`

It demonstrates `List`, `Enum`, `Map`, and non-interactive `IO.inspect/1` together under the same project-mode lazy-loading contract the docs describe.

## Summary

The current honest Tonic Core Stdlib profile is:

- **Core-supported host-backed modules:** `System`, `String`, `Path`, `IO`
- **Core-supported pure collection module:** `List`
- **Core-supported bounded collection modules:** `Map` and `Enum`, with the documented host/pure split
- **Deferred:** broader Elixir-shaped utility modules until demand, runtime support, and parity justify them
- **Still caveated:** optional stdlib injection remains project-mode-only

That is a much stronger claim than the older `System`/`String`/`Path`-only baseline, but it is still intentionally narrower than “Elixir stdlib compatibility”. The point is not breadth. The point is a supported surface users can actually rely on.
