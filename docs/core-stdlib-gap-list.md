# Tonic Core Stdlib Gap List

Status: prioritized follow-up list after `docs/core-stdlib-profile.md` and `docs/text-binary-parser-contract.md`  
Last updated: 2026-03-07

This document records the main remaining gaps between Tonic's current stdlib reality and the more coherent, Elixir-shaped core profile Tonic wants for real app authoring.

Read this alongside:

- [core-stdlib-profile.md](core-stdlib-profile.md)
- [text-binary-parser-contract.md](text-binary-parser-contract.md)
- [app-authoring-gaps.md](app-authoring-gaps.md)
- `.agents/planning/2026-03-07-elixir-core-stdlib/sitegen-stdlib-audit.md`

The rule for this list is simple:

- prefer **honest support** over broad advertising,
- prioritize **workload-backed surfaces**,
- treat **interpreter/native parity + docs + tests** as the support bar.

---

## Closure update

The previous P0 honesty gap is now closed for the worst offenders:

- `Enum`
- `List`
- `IO`
- `Map`

They were removed from lazy project-mode stdlib injection in `src/manifest.rs`, and the optional injected surface is now honestly limited to:

- `System`
- `String`
- `Path`

That does **not** mean the four removed modules are implemented. It means they are no longer advertised by default before parity exists.

---

## Priority summary

| Priority | Gap | Why it matters | Recommended next move |
|---|---|---|---|
| P1 | Optional stdlib injection is still project-mode-only | `tonic run file.tn` and project mode do not see the same stdlib surface, which makes examples and mental models inconsistent | Decide whether to unify single-file + project-mode stdlib behavior or document the split as an intentional product tier |
| P1 | Runtime text is binary-shaped, not parser-ready byte/list input | Real app authoring still lacks a polished parser-oriented text contract beyond `String.*` helpers | Either keep the current contract and add explicit parser-friendly helpers, or deliberately improve runtime text decomposition semantics |
| P2 | Deferred stdlib modules still have dormant in-tree implementations without parity | `Enum`, `List`, `IO`, and `Map` now avoid misleading injection, but the repo still contains partial interpreter-side work that could be mistaken for support | Keep them de-advertised until end-to-end registration, native dispatch, tests, and docs are all real |
| P2 | Filesystem ownership is still System-heavy rather than Elixir-shaped | Tonic now has useful filesystem primitives, but their module placement is still more pragmatic than elegant | Keep under `System` for now, but document whether a future `File` split is intended |
| P2 | Several Elixir-shaped modules are still absent or deferred (`URI`, `Keyword`, `Integer`, `Float`, `Tuple`, `OptionParser`, `Regex`, `Stream`) | The profile is still intentionally narrow | Add only when workload evidence exists and the runtime contract is clear |

---

## Closed gap — injected broken modules were de-advertised

### What changed

`src/manifest.rs` no longer lazy-loads:

- `Enum`
- `List`
- `IO`
- `Map`

Repo-local smoke coverage now locks the new behavior:

- supported injected modules (`System`, `String`, `Path`) still lazy-load in project mode
- deferred modules no longer lazy-load implicitly
- references to deferred modules now fail honestly as undefined symbols instead of reaching misleading unknown-host or parse-hostile injected-module failures

### Why this was the right closure slice

Fresh evidence still showed:

- no workload pressure from `tonic-sitegen-stress`
- no end-to-end host registration in `src/interop.rs`
- no `enum_*`, `list_*`, `io_*`, or `map_*` native dispatch in `src/c_backend/stubs.rs`
- `Map` injected source was still parse-hostile because of `has_key?`

So the smallest honest move was not “implement a thin slice badly.” It was “stop auto-advertising unsupported modules.”

### Remaining follow-up for those modules

If any of these return to the injected stdlib surface later, the bar should be:

1. interpreter registration exists
2. native compiled dispatch exists
3. regression coverage exists
4. docs/profile are updated at the same time

Until then, they should stay deferred.

---

## P1 — Execution-mode split: project mode vs single-file mode

### Current state

Optional stdlib injection currently happens in **project mode** but not in plain single-file execution.

That means:
- `tonic run <project-dir>` can lazy-load optional stdlib modules
- `tonic compile <project-dir>` can do the same
- `tonic run file.tn` does **not** receive that optional stdlib surface

### Why it matters

This creates a product-model gap:
- examples may behave differently depending on how they are invoked
- users cannot assume `String`, `System`, `Path`, etc. exist in single-file mode
- it weakens the “small coherent core stdlib” story

### Recommended outcome

The next stdlib loops should explicitly decide one of:

1. **Unify behavior** — optional stdlib works in both project and single-file modes
2. **Document the split as intentional** — project mode gets the optional stdlib, single-file mode does not

What should not remain indefinitely:
- accidental split behavior without a product-level explanation

---

## P1 — Parser-oriented runtime text ergonomics

### Current state

The text/binary parser contract is now documented honestly:
- runtime text is `is_binary: true`
- runtime text is `is_list: false`
- list-prefix and `<<...>>` byte parsing are not a supported runtime-text path
- `String.*` is the supported near-term parser-ish path

### Why it still counts as a gap

The contract is honest now, but still not ideal for parser-heavy workloads.

A more Elixir-shaped app-authoring story would eventually need one of:
- explicit parser-friendly byte/char iteration helpers
- explicit string-to-byte decomposition helpers
- stronger, intentionally supported runtime-text matching semantics

### Recommended outcome

Treat this as a separate design/implementation loop after the injected broken-module problem is closed.

---

## P2 — Deferred modules with dormant partial implementations

### Current state

The repo still contains interpreter-side host modules for:

- `Enum`
- `List`
- `IO`
- `Map`

But these modules are not currently part of the honest optional stdlib surface.

### Why this is only P2 now

This is no longer a user-facing honesty failure because the modules are not injected by default anymore.

It is still worth tracking because dormant code can create false confidence during future implementation work.

### Recommended outcome

Do not re-advertise these modules piecemeal.

Only bring one back when:
- workload demand exists
- interpreter wiring is complete
- native parity is complete
- regression tests land in the same slice

---

## P2 — Filesystem module shape (`System` vs future `File`)

### Current state

Useful filesystem primitives now exist under `System`, including:
- `System.list_files_recursive/1`
- `System.remove_tree/1`
- `System.path_exists/1`
- `System.ensure_dir/1`
- `System.read_text/1`
- `System.write_text/2`

### Why this is still a gap

For an Elixir-shaped stdlib, these functions might eventually fit better under a `File` module, with `System` focused more on process/environment/runtime interactions.

### Recommended outcome

Do not rename for aesthetics yet.

Instead:
- keep the current `System` surface usable
- document the likely long-term direction
- revisit only when workload pressure justifies it

---

## P2 — Deferred Elixir-shaped modules

These are reasonable future candidates, but they are not currently part of the supported core stdlib profile:

- `URI`
- `Keyword`
- `Integer`
- `Float`
- `Tuple`
- `OptionParser`
- `Regex`
- `Stream`

Do not broaden into these until the higher-priority gaps above are closed.

---

## Recommended next loop focus

The next Ralph loop should focus on one of these, in order:

1. resolve the execution-mode split for optional stdlib injection
2. improve parser-oriented text ergonomics without pretending text is already a parser-ready byte/list type
3. reintroduce any deferred stdlib module only when parity work is real

The point is the same as before: keep the stdlib story smaller than users might want, but truer than it was before.
