# Tonic Core Stdlib Profile

Status: draft, reviewable baseline  
Last updated: 2026-03-07

This document defines the **Tonic Core Stdlib** that Tonic intends to support for real app authoring.

The goal is not to clone Elixir's full standard library. The goal is to provide a **small, honest, Elixir-shaped subset** that works across Tonic's interpreter and native compiled runtime, is driven by real workloads, and documents intentional divergence where Tonic is not BEAM.

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

## Evidence base for this profile

This baseline is grounded in:

- `docs/system-stdlib.md`
- `docs/app-authoring-gaps.md`
- `PARITY.md`
- `src/manifest.rs`
- `src/interop.rs`
- `src/interop/system.rs`
- `src/interop/string_mod.rs`
- `src/interop/path_mod.rs`
- `src/c_backend/stubs.rs`
- `.agents/planning/2026-03-07-elixir-core-stdlib/sitegen-stdlib-audit.md`
- `/home/mobrienv/projects/tonic-sitegen-stress`

The main workload driver is `tonic-sitegen-stress`, which currently shows a narrow but real need centered on:

- `String.split/2`
- `String.trim/1`
- `String.starts_with/2`
- `String.to_integer/1`
- `System.path_exists/1`
- `System.list_files_recursive/1`
- `System.remove_tree/1`
- `System.ensure_dir/1`
- `System.read_text/1`
- `System.write_text/2`
- `System.argv/0`

That workload does **not** currently justify a broad public promise for `Enum`, `List`, `Map`, `IO`, `URI`, `Keyword`, `Integer`, `Float`, `Tuple`, or `OptionParser`.

## Core profile rules

A stdlib surface belongs in the Tonic Core Stdlib only if all of the following are true:

1. **Real demand exists** from app-authoring workloads, examples, or core tooling.
2. **Interpreter behavior exists** and is intentional.
3. **Native compiled behavior exists** and matches the intended contract closely enough to advertise.
4. **Regression coverage exists** for the supported behavior.
5. **Divergence is documented** where Tonic does not match Elixir semantics.

If any of those are missing, the surface may be:

- present in source,
- injected by `manifest.rs`, or
- experimentally usable,

but it is **not yet part of the supported core profile**.

## Support labels used in this document

| Label | Meaning |
|---|---|
| **Core-supported** | Workload-backed, interpreter + native supported, documented, and regression-covered |
| **Available but secondary** | End-to-end working today, but not currently central to the main app-authoring workload |
| **Advertised ahead of support** | Exposed by stdlib injection or docs, but not yet honest to treat as supported |
| **Deferred** | Not currently part of the supported optional stdlib surface or not yet justified by workload |

## Current profile baseline

### 1. Core-supported now

These are the modules Tonic should treat as the current core profile baseline.

#### `String`

`String` is the main text-processing surface the current workload actually needs.

Current workload-proven subset:

- `String.split/2`
- `String.trim/1`
- `String.trim_leading/1`
- `String.trim_trailing/1`
- `String.starts_with/2`
- `String.ends_with/2`
- `String.contains/2`
- `String.slice/3`
- `String.to_integer/1`

The injected `String` module currently exposes a broader set than that, but the profile should center the subset above because it is both real and covered.

#### `System`

`System` is the other core module today. In current Tonic, it is the practical home for:

- filesystem operations
- process execution
- environment access
- CLI input/output helpers
- HTTP client/server helpers
- selected crypto helpers

For the app-authoring core profile, the important currently proven subset is:

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

Not every existing `System.*` helper is equally central to the profile, but this module is real, heavily host-backed, and already the main app-authoring system boundary.

### 2. Available but secondary

#### `Path`

`Path` currently works in interpreter and native compiled execution and should be treated as **available**, but not as the center of the profile yet.

Reason:

- it is real today
- it has parity evidence
- but `tonic-sitegen-stress` currently does not depend on it in non-probe source

Current stance:

- `Path` is part of the honest usable surface
- `Path` is not yet the reason to define the core profile
- future workloads may promote it

### 3. Advertised ahead of support

These modules are currently injected by `src/manifest.rs` but should **not** be treated as part of the supported core stdlib profile:

- `Enum`
- `List`
- `Map`
- `IO`

Current status from the audit:

| Module | Current status | Why it is not in the core profile |
|---|---|---|
| `Enum` | broken in project mode at runtime | host functions are not wired end to end |
| `List` | broken in project mode at runtime | host functions are not wired end to end |
| `IO` | broken in project mode at runtime | host functions are not wired end to end |
| `Map` | broken before runtime in injected source | injected stdlib source is not even parse-safe today |

Policy consequence: these modules may exist in `manifest.rs`, but they should be treated as **pre-profile / not yet supported** until interpreter + native support and regression coverage exist.

### 4. Explicitly deferred

These are reasonable future candidates, but they are not part of the current Tonic Core Stdlib profile:

- `URI`
- `Keyword`
- `Integer`
- `Float`
- `Tuple`
- `OptionParser`
- `File` as a separate module split from `System`
- `Regex`
- `Stream`

Reason for deferment:

- they are not currently part of the injected optional stdlib surface, or
- the current workload does not yet justify them, or
- they depend on a clearer runtime contract first

## Parity policy

A stdlib function should be described as **supported** only when all of the following are true:

1. **Interpreter support exists** for the intended argument and error contract.
2. **Native compiled support exists** for the same contract.
3. **Any known divergence is documented** in the relevant doc.
4. **Regression coverage exists** in repo tests.
5. **Advertising matches reality** across docs, manifest injection, and examples.

A function should **not** be broadly advertised as supported when any of these are true:

- only the interpreter works
- only the native path works
- the wrapper exists but the host registration does not
- the module injects but fails to parse
- the docs describe behavior that no parity test covers

## Execution-mode caveat

Current important limitation: optional stdlib injection happens in **project mode**, not in plain single-file execution.

Today this means:

- `tonic run <project-dir>` and `tonic compile <project-dir>` can lazy-load optional stdlib modules from `src/manifest.rs`
- `tonic run file.tn` does **not** currently get that optional stdlib injection

So the current stdlib contract is not just module-dependent; it is also **execution-mode-dependent**.

This caveat must remain explicit until single-file and project-mode behavior are intentionally unified or intentionally documented as different product tiers.

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

Tonic also currently diverges in a more immediate app-authoring area:

### Text has a documented binary-shaped contract, not a parser-ready byte/list contract

The current contract is defined in [text-binary-parser-contract.md](text-binary-parser-contract.md).

Current runtime reality is still:

- text values behave as binary-shaped runtime data
- `is_binary(text)` is true and `is_list(text)` is false for runtime text
- parser-style list-prefix and bitstring-byte matching are not a supported contract for runtime text
- workload-shaped text parsing should use the workload-backed `String` helpers instead

That is still the biggest remaining product-shape gap exposed by `tonic-sitegen-stress`.

This means the right near-term move is **not** to paper over the gap by broadening module count. The right move is to keep the text/binary/parser contract honest and improve it deliberately.

## Module direction notes

### Why filesystem stays under `System` for now

Elixir would normally push more filesystem operations toward `File`, but Tonic should avoid churn for its own sake.

Current stance:

- keep the working filesystem surface under `System` for now
- document it honestly there
- consider a future `File` split or alias only after the profile is otherwise stable

The profile goal is honest capability, not naming cosplay.

## Current status matrix

| Module/surface | Profile status | Evidence summary |
|---|---|---|
| `String` workload subset | Core-supported | real sitegen demand + interpreter/native proof |
| `System` workload subset | Core-supported | real sitegen demand + interpreter/native proof |
| `Path` | Available but secondary | working today, but not central to current workload |
| `Enum` | Advertised ahead of support | injected but missing end-to-end host support |
| `List` | Advertised ahead of support | injected but missing end-to-end host support |
| `IO` | Advertised ahead of support | injected but missing end-to-end host support |
| `Map` | Advertised ahead of support | injected source still not parse-safe |
| `URI` / `Keyword` / `Integer` / `Float` / `Tuple` / `OptionParser` | Deferred | not current optional-stdlib reality and not yet workload-driven |

## What this profile means for future work

Near-term stdlib work should follow this order:

1. keep docs honest about the current profile boundary
2. stop treating injected-but-broken modules as supported
3. define the text/binary/parser contract explicitly
4. add new stdlib surface only when there is workload demand plus parity evidence

The first likely high-value follow-up after this profile is **text/binary/parser ergonomics**, not a broad module expansion.

## Summary

The current honest Tonic Core Stdlib profile is:

- **Core-supported:** `String` and `System`, using the workload-proven subsets
- **Available but secondary:** `Path`
- **Not profile-ready despite injection:** `Enum`, `List`, `IO`, `Map`
- **Deferred:** broader Elixir-shaped utility modules until workload and parity justify them

That is a smaller claim than “Tonic has an Elixir-like stdlib,” but it is the right one. It gives app authors a clearer answer about what they can rely on today, and it sets a better bar for every stdlib surface added next.
