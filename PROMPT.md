# Tonic Stdlib Usability Push

## Objective

Implement the stdlib/runtime work needed to make Tonic materially more usable for real programs.

This loop should take the current state from:

- core data structures exist but stdlib exposure is uneven,
- some stdlib modules already have host-side implementations but are de-advertised,
- the pure-vs-host split is unclear,
- and native/backend parity is incomplete,

to a state where:

- the right modules are exposed,
- pure transforms are implemented in Tonic where appropriate,
- side-effecting/runtime-sensitive features are host-backed,
- map functionality is handled honestly,
- and run/compile/docs/tests all agree.

## Product position

Do **not** treat this as a vague stdlib expansion.
Treat it as a concrete usability milestone with an explicit split:

### Must remain primitive / host-backed

- `IO.*`
- `System.*`
- most or all of `Path.*`
- UTF-8/parsing-heavy `String.*`
- crypto/networking/process/time/filesystem primitives

### Should be implemented directly in Tonic

- `List.*` helpers that are pure structural transforms
- `Enum.*` helpers over lists/ranges that are pure transforms

### Needs honest handling because of current language limitations

- `Map.*`
- any `Enum.*` behavior requiring generic map traversal

If you cannot implement a clean pure-Tonic `Map` story without adding new language/runtime support, do **not** fake it. Either:

1. add the minimum bounded traversal support required and prove it.
2. Implement the `Map` story.

Prefer the smallest truthful design that lands usable functionality.

## Source-of-truth context

Read these first:

- `AGENTS.md`
- `.agents/summary/index.md`
- `.agents/summary/interfaces.md`
- `.agents/summary/components.md`
- `src/manifest.rs`
- `src/interop.rs`
- `src/interop/system.rs`
- `src/interop/string_mod.rs`
- `src/interop/path_mod.rs`
- `src/interop/io_mod.rs`
- `src/interop/list_mod.rs`
- `src/interop/map_mod.rs`
- `src/interop/enum_mod.rs`
- `src/c_backend/stubs.rs`
- `tests/run_lazy_stdlib_loading_smoke.rs`
- `tests/run_comprehensions_smoke.rs`
- `tests/run_case_list_map_smoke.rs`
- `tests/system_stdlib_http_input_smoke.rs`
- `examples/parity/10-idiomatic/list_processing.tn`
- `examples/parity/10-idiomatic/pipeline_transform.tn`
- `examples/parity/10-idiomatic/closures_and_captures.tn`

Use repo source as truth if docs and implementation diverge. Update docs as you progress.

## Problem statement

Tonic already has more capability than its public stdlib surface suggests.

Today:

- lists and maps already exist as runtime values,
- pattern matching and comprehensions already exist,
- `System`, `String`, and `Path` are lazily exposed,
- host implementations for `IO`, `List`, `Map`, and `Enum` already exist in Rust,
- but those modules are not properly exposed/wired,
- and there are clear backend-parity gaps.

This creates the worst combination:

- useful capabilities exist,
- users cannot rely on them consistently,
- and the implementation boundary is muddier than it should be.

## Goals

### Primary goals

1. Expose the right stdlib modules.
2. Implement pure collection transforms directly in Tonic where feasible.
3. Keep side-effecting/runtime-sensitive surfaces host-backed.
4. Handle `Map.*` honestly given current traversal limitations.
5. Ensure interpreter, C backend, docs, and tests all agree.

### Secondary goals

- improve self-hosting posture by moving pure library logic into Tonic where appropriate,
- reduce unnecessary Rust glue for pure transforms,
- keep file sizes reasonable,
- and leave a clear boundary between library code and primitives.

## Scope

### In scope

- exposing `IO`, `List`, `Map`, and `Enum` as public stdlib modules
- deciding and implementing the correct pure-vs-host split
- implementing pure `List.*` and `Enum.*` in Tonic if the language supports them cleanly
- keeping `IO.*`, `System.*`, and other external/runtime-sensitive features host-backed
- wiring lazy stdlib loading for newly exposed modules
- registering all required host functions
- adding or completing native/C-backend host-call parity for host-backed surfaces
- updating docs generation so stdlib docs reflect the real exposed surface
- replacing de-advertising tests with truthful positive/negative coverage
- adding representative runtime and native parity tests

### Out of scope unless strictly required

- a giant general-purpose stdlib redesign
- speculative optimizer work
- parser/typechecker rewrites unrelated to this milestone
- broad self-hosting work beyond pure stdlib modules
- ambitious map-iteration language design beyond the minimum bounded support needed

## Required module split

Implement using this split unless source evidence proves a better bounded alternative.

### Host-backed modules/surfaces

#### `IO`

Expose at least:

- `IO.puts/1`
- `IO.inspect/1`
- `IO.gets/1`
- `IO.ansi_red/1`
- `IO.ansi_green/1`
- `IO.ansi_yellow/1`
- `IO.ansi_blue/1`
- `IO.ansi_reset/0`

These should remain host-backed.

#### `System`

Keep host-backed. Do not attempt to reimplement external effects in Tonic.

#### `Path`

Keep host-backed unless a function is trivial and clearly benefits from being pure.
Default to host-backed.

#### `String`

Keep existing host-backed UTF-8/parsing-sensitive behavior unless there is a compelling reason otherwise.

#### `Map`

Expose a usable `Map` module.

Default acceptable implementation for this milestone:

- host-backed `Map.keys/1`
- `Map.values/1`
- `Map.merge/2`
- `Map.drop/2`
- `Map.take/2`
- `Map.has_key?/2`
- `Map.get/3`
- `Map.put/3`
- `Map.delete/2`

Do **not** pretend `Map.filter/2` or `Map.reject/2` are production-ready if they still depend on missing runtime closure dispatch or missing traversal support. 

### Pure-Tonic modules/surfaces

#### `List`

Implement directly in Tonic if cleanly possible:

- `List.first/1`
- `List.last/1`
- `List.wrap/1`
- `List.flatten/1`
- `List.zip/2`
- `List.unzip/1`

#### `Enum`

Implement directly in Tonic over lists/ranges if cleanly possible:

- `Enum.count/1`
- `Enum.sum/1`
- `Enum.join/2`
- `Enum.sort/1`
- `Enum.reverse/1`
- `Enum.take/2`
- `Enum.drop/2`
- `Enum.chunk_every/2`
- `Enum.unique/1`
- `Enum.into/2` for list targets and any clearly supported target types
- high-value transforms if feasible and safe:
  - `Enum.map/2`
  - `Enum.filter/2`
  - `Enum.reduce/3`

#### 1. External side effects require host/runtime primitives

Pure Tonic cannot directly implement:

- stdin/stdout/stderr
- files
- env/argv
- networking
- time/sleep
- crypto/randomness
- process spawning

So `IO.*` and `System.*` are not candidates for a pure-Tonic implementation.

#### 2. Generic map traversal appears missing from the current public language surface

There does not appear to be a clean first-class way to:

- iterate map entries directly in Tonic,
- convert a map into an entry list in pure Tonic,
- or fold over a map generically.

That blocks or complicates pure implementations of much of `Map.*`.

### Near-blockers / engineering risks

- recursion depth and performance for pure library code
- closure-heavy parity across interpreter/C/LLVM backends
- sorting semantics if implemented purely in Tonic
- any attempt to widen map iteration beyond a small bounded change

## Delivery order

Work in this order unless a small reorder clearly improves safety.

### Phase 1 — expose real surfaces and remove dead plumbing ambiguity

1. audit current stdlib exposure and host registration
2. wire `IO`, `List`, `Map`, and `Enum` into the exposed stdlib surface
3. update lazy-loading analysis so newly exposed modules load when referenced
4. update docs-generation inputs so generated stdlib docs match reality
5. replace the current de-advertising expectations with truthful tests

### Phase 2 — land the correct implementation split

6. move pure `List.*` into Tonic source if feasible and cleaner than Rust-backed wrappers
7. move pure `Enum.*` into Tonic source where the implementation is clean and parity-safe
8. keep `IO.*`, `System.*`, `Path.*`, and relevant `String.*` host-backed
9. implement `Map.*` as host-backed unless you first add the minimum bounded traversal support required to do better

### Phase 3 — native/backend parity

10. ensure every host-backed function exposed publicly has matching native/C-backend support in `src/c_backend/stubs.rs`
11. verify interpreter and compiled behavior agree for representative cases
12. make sure LLVM/C host-call paths do not silently lag the interpreter surface

### Phase 4 — polish and docs

13. add representative examples demonstrating the final public surface
14. update docs/comments/help text if the exposed stdlib changes
15. leave explicit notes for any intentionally deferred APIs

## Concrete implementation targets

### `src/manifest.rs`

Update embedded stdlib sources and module lists so the exposed surface is real.
This likely includes:

- adding embedded source for `IO`, `List`, `Map`, and `Enum`, or
- moving pure-Tonic stdlib module sources into a cleaner arrangement if appropriate,
- updating `STDLIB_SOURCES`,
- updating `STDLIB_MODULE_NAMES`,
- updating the lazy-load module set in `load_run_source_from_project_root`.

### `src/interop.rs`

Wire in modules that already exist but are not registered:

- `io_mod`
- `list_mod`
- `map_mod`
- `enum_mod`

Register the host functions that remain part of the public host-backed surface.
If you move some modules to pure Tonic, remove or narrow dead Rust-side plumbing so there is no misleading unused implementation.

### `src/interop/*.rs`

Use these files as source of truth for host-backed behavior.
Refine them as needed, but do not leave dead exported surfaces that the language never exposes.

### `src/c_backend/stubs.rs`

Add host-call parity for every public host-backed function you expose.
Do not ship interpreter-only stdlib behavior by accident.

### tests

Update and extend:

- `tests/run_lazy_stdlib_loading_smoke.rs`
- targeted runtime smoke tests
- targeted compile/native parity tests
- deterministic negative tests for intentionally unsupported APIs

### examples/docs

Add or refresh small examples so the public surface is discoverable and believable.

## Minimum acceptance matrix

At completion, the following should be true.

### Exposure / usability

- `IO.inspect(123)` works when `IO` is referenced
- `List.first([1, 2, 3])` works
- `Map.keys(%{a: 1})` works
- `Enum.count([1, 2, 3])` works
- stdlib docs reflect the exposed modules

### Purity split

- pure transforms live in Tonic where cleanly feasible
- side-effecting/runtime-sensitive behavior remains host-backed
- `Map.*` is handled honestly rather than by wishful abstraction

### Backend parity

- representative host-backed APIs work in interpreter mode
- representative host-backed APIs work in compiled/native mode
- no public stdlib function is interpreter-only unless explicitly documented and tested as unsupported elsewhere

### Quality

- no dead plumbing left behind without reason
- no misleading tests asserting deliberate non-exposure of modules that are now public
- no warnings or obviously stale code paths

## Specific verification expectations

Run the smallest sufficient checks during each slice, then a focused final gate.

At minimum, expect to run relevant subsets of:

- `cargo test --test run_lazy_stdlib_loading_smoke`
- `cargo test --test run_comprehensions_smoke`
- `cargo test --test run_case_list_map_smoke`
- `cargo test --test run_pipe_enum_smoke`
- `cargo test --test system_stdlib_http_input_smoke`
- targeted new runtime tests for `IO`, `List`, `Map`, and `Enum`
- targeted compile/native parity tests for exposed host-backed surfaces

If the touched surface is broad enough, run:

- `cargo test`

If you modify C backend host-call support, prove it with at least one compiled/native test that exercises the new public surface.

## Constraints

- Keep implementation files reasonably small; if a file grows past the repo preference, split it.
- Prefer pure Tonic for pure transforms, but do not force bad implementations just to avoid Rust.
- Do not claim pure `Map.*` support unless you truly added the required language/runtime support.
- Do not leave `Map.filter/2` and `Map.reject/2` pretending to be complete if they are not.
- Treat dead code and warnings as blockers.
- Commit when tests pass.

## Definition of done

Emit `LOOP_COMPLETE` only when:

- `IO`, `List`, `Map`, and `Enum` are exposed intentionally and truthfully,
- the pure-vs-host split is implemented rather than merely discussed,
- pure `List`/`Enum` functionality is in Tonic where feasible,
- side-effecting/runtime-sensitive APIs remain host-backed,
- `Map.*` is usable and honestly scoped,
- native/backend parity exists for every public host-backed API shipped in this milestone,
- tests and docs reflect the real surface,
- and the repo is left in a cleaner, more credible usability state than before.
- Examples exist that exercise the implementation.
- Documentation has been updated.
