# Tonic Core Stdlib Gap List

Status: prioritized follow-up list after the stdlib usability push  
Last updated: 2026-03-09

This document records the main remaining gaps between Tonic's current stdlib reality and the more coherent, Elixir-shaped core profile Tonic wants for real app authoring.

Read this alongside:

- [core-stdlib-profile.md](core-stdlib-profile.md)
- [text-binary-parser-contract.md](text-binary-parser-contract.md)
- [app-authoring-gaps.md](app-authoring-gaps.md)

The rule for this list is simple:

- prefer **honest support** over broad advertising
- prioritize **parity-backed surfaces** over wishlist APIs
- treat **docs + examples + interpreter + native tests** as the support bar

---

## Closure update

The previous usability milestone is now materially closed for the collection/IO surface.

The current optional project-mode stdlib surface exposed by `src/manifest.rs` is now:

- `System`
- `String`
- `Path`
- `IO`
- `List`
- `Map`
- `Enum`

That exposed surface now matches the implemented split:

- `IO`, `System`, `Path`, and `String` stay host-backed
- `List` lives in injected Tonic source
- `Enum` is mostly pure Tonic with bounded host-backed helpers for `join/2` and `sort/1`
- `Map` is exposed as the bounded host-backed surface Tonic can honestly support today

This is no longer the old de-advertise-everything phase. The remaining gaps are the next-order gaps after parity and exposure landed.

---

## Priority summary

| Priority | Gap | Why it matters | Recommended next move |
|---|---|---|---|
| ~~P1~~ | ~~Optional stdlib injection is still project-mode-only~~ | Resolved: single-file and project mode now share the same stdlib injection surface | Unified in `load_run_source` — both paths call `inject_optional_stdlib` |
| ~~P1~~ | ~~Runtime text is still binary-shaped, not parser-ready byte/list input~~ | Documented design decision: Tonic uses binary strings with `String.*` helpers for text manipulation. Not a BEAM runtime — byte/list text decomposition is intentionally out of scope. | See [text-binary-parser-contract.md](text-binary-parser-contract.md) |
| P2 | Public collection surface is intentionally bounded | Tonic now has a believable `List`/`Enum`/`Map` story, but some obvious Elixir-shaped helpers still remain deferred for honest reasons | Expand only one bounded helper at a time, with native parity and docs in the same slice |
| P2 | Filesystem ownership is still `System`-heavy rather than Elixir-shaped | The filesystem surface is usable, but the module shape is still pragmatic rather than elegant | Keep it under `System` for now; revisit a future `File` split only if workload pressure justifies it |
| P3 | Several Elixir-shaped modules are still absent or deferred (`URI`, `Keyword`, `Integer`, `Float`, `Tuple`, `OptionParser`, `Regex`, `Stream`) | The profile is still intentionally narrow | Add only when workload evidence exists and the runtime contract is clear |

---

## Closed gap — exposed surface now matches real support

### What landed

The stdlib usability push now leaves Tonic in a much cleaner state:

- `IO`, `List`, `Map`, and `Enum` are public again in project mode
- pure collection transforms moved into injected Tonic source where that was the honest design
- public host-backed `Map.*` and `IO.*` helpers now have native/C-backend parity
- lazy-load smoke coverage proves the exposed modules actually load when referenced
- focused native smokes prove the public host boundary no longer silently lags the interpreter
- docs and examples can now describe the real project-mode surface instead of a narrower placeholder profile

### Why this closure matters

Before this work, Tonic had the worst combination:

- useful capabilities existed in-tree
- users could not rely on them end to end
- and the implementation boundary was muddy

Now the boundary is much clearer:

- pure transforms live in Tonic where they should
- runtime-sensitive features stay host-backed
- `Map` is exposed honestly as a bounded surface
- native parity exists for the shipped public host-backed helpers

### What is still intentionally missing

The current closure does **not** mean “Tonic now has Elixir stdlib parity”.

The still-deferred edges include:

- `Map.has_key?/2` — parser/public-surface issue around `?`-suffixed names
- `Map.filter/2` and `Map.reject/2` — not yet a strong enough public traversal/closure story to advertise honestly
- `Enum.map/2`, `Enum.filter/2`, `Enum.reduce/3` — valuable candidates, but not part of the current public contract yet
- broad module expansion beyond the current seven-module surface

Those are now feature decisions, not honesty repairs.

---

## P1 — Execution-mode split: project mode vs single-file mode

### Current state

Optional stdlib injection still happens in **project mode** but not in plain single-file execution.

That means:

- `tonic run <project-dir>` can lazy-load optional stdlib modules
- `tonic compile <project-dir>` can do the same
- `tonic run file.tn` does **not** receive that optional stdlib surface

### Why it matters

This is now the sharpest remaining product-model gap:

- examples must keep spelling out the project-mode caveat
- users cannot assume the same stdlib surface across `run` entry modes
- the current core profile is real, but only within the project-mode contract

### Recommended outcome

The next stdlib loop should explicitly decide one of:

1. **Unify behavior** — optional stdlib works in both project and single-file modes
2. **Document the split as intentional** — project mode gets the optional stdlib, single-file mode does not

What should not remain indefinitely is accidental split behavior without a product-level explanation.

---

## P1 — Parser-oriented runtime text ergonomics

### Current state

The text/binary parser contract is documented honestly:

- runtime text is `is_binary: true`
- runtime text is `is_list: false`
- list-prefix and `<<...>>` byte parsing are not a supported runtime-text path
- `String.*` is the supported near-term parser-ish path

### Why it still counts as a gap

The contract is honest now, but parser-heavy workloads still need a stronger story than “use `String.*` and be careful”.

A more polished app-authoring story would eventually need one of:

- explicit parser-friendly byte or char iteration helpers
- explicit string-to-byte decomposition helpers
- intentionally supported runtime-text matching semantics

### Recommended outcome

Treat this as a separate design loop. Do not blur it together with collection-surface work.

---

## P2 — Bounded collection surface still leaves obvious follow-ups

### Current state

The current public collection surface is intentionally modest:

- `List`: `first`, `last`, `wrap`, `flatten`, `zip`, `unzip`
- `Enum`: `count`, `sum`, `join`, `sort`, `reverse`, `take`, `drop`, `chunk_every`, `unique`, `into`
- `Map`: `keys`, `values`, `merge`, `drop`, `take`, `get`, `put`, `delete`

### Why this is still a gap

Users will reasonably look for helpers like:

- `Map.has_key?/2`
- `Enum.map/2`
- `Enum.filter/2`
- `Enum.reduce/3`

Those are not blocked because the current surface is broken. They are deferred because the next step should stay honest and bounded.

### Recommended outcome

Only expand this surface when each added helper ships with:

1. a clear implementation boundary
2. interpreter coverage
3. native parity where required
4. docs and examples updated in the same slice

---

## P2 — Filesystem module shape (`System` vs future `File`)

### Current state

Useful filesystem primitives still live under `System`, including:

- `System.list_files_recursive/1`
- `System.remove_tree/1`
- `System.path_exists/1`
- `System.ensure_dir/1`
- `System.read_text/1`
- `System.write_text/2`

### Why this is still a gap

For a more Elixir-shaped stdlib, these functions might eventually fit better under a `File` module, with `System` focused on process/environment/runtime interactions.

### Recommended outcome

Do not rename for aesthetics yet.

Instead:

- keep the current `System` surface usable
- keep docs honest about where filesystem lives today
- revisit a future `File` split only if workload pressure justifies it

---

## P3 — Deferred Elixir-shaped modules

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

## Example reference

For the small project-mode showcase that now matches the documented surface, see:

- `examples/apps/stdlib_showcase`

It is intentionally a project example, not a single-file example, because the current stdlib contract still depends on project-mode lazy loading.

---

## Recommended next loop focus

The next stdlib loop should focus on one of these, in order:

1. resolve the execution-mode split for optional stdlib injection
2. improve parser-oriented text ergonomics without pretending text is already a parser-ready byte/list type
3. expand the bounded collection surface one helper at a time, only when parity and docs land together

The current stdlib story is finally believable. The next work should keep it that way.
