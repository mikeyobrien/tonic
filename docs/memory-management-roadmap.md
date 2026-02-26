# Runtime Memory Management Roadmap (Research + Scaffold)

_Last updated: 2026-02-26_

## Why this exists

Tonic is already usable for real app development, but memory behavior differs by runtime path:

- `tonic run` (Rust interpreter) relies on Rust ownership and process lifetime.
- Native ABI (`src/native_abi/*`) already has handle-based refcounting.
- Generated C runtime (`src/c_backend/stubs.rs`) currently uses an append-only boxed heap table (`tn_heap_store`) with no full object reclamation pass.

For short-lived CLI usage this is acceptable; for long-running services it is a risk.

This document compares practical approaches and scaffolds an incremental execution plan.

---

## Current state (code anchors)

### 1) Interpreter path (`src/runtime.rs`)
- Values are Rust enums/containers (`RuntimeValue`) and drop naturally.
- No custom GC needed in interpreter mode.

### 2) Native ABI path (`src/native_abi/mod.rs`, `src/native_abi/heap.rs`)
- `TValue` uses immediate or refcounted handle payloads.
- `retain_tvalue` / `release_tvalue` manage handle refs.
- Deterministic errors for invalid handles and ownership violations.

### 3) Generated C runtime path (`src/c_backend/stubs.rs`)
- Boxed values stored in global `tn_heap` via `tn_heap_store`.
- Heap grows by `realloc` and object pointers remain until process exit.
- Some temporary C buffers are freed (`free(args)`, etc.), but boxed runtime objects are not globally reclaimed.

---

## Constraints for a good solution

1. Keep startup latency low for CLI workflows.
2. Preserve deterministic runtime errors/diagnostics.
3. Minimize codegen disruption in early phases.
4. Support long-running processes without unbounded growth.
5. Keep generated runtime understandable and testable.

---

## Candidate approaches

### A) Observability + guardrails first (no reclamation yet)

### What it is
Add memory counters, snapshots, and budget checks before changing ownership semantics.

### Scope
- Allocation counters per runtime kind.
- High-water mark tracking.
- Optional fail-fast budget env vars (e.g. `TONIC_MEMORY_LIMIT_MB`).
- Deterministic diagnostics when budget is exceeded.

### Pros
- Lowest risk.
- Immediately improves operability.
- Provides data for choosing RC vs tracing.

### Cons
- Does not reclaim memory.

### Complexity
Low.

---

### B) Region/arena reset model (phase-friendly)

### What it is
Allocate runtime objects in one or more arenas and reclaim whole arena(s) at safe boundaries.

### Typical boundaries
- Per top-level command/evaluation.
- Optional sub-arena per test case/comprehension/closure batch.

### Pros
- Very low runtime overhead.
- Minimal write-barrier complexity.
- Good match for batch/CLI execution.

### Cons
- Poor fit for long-lived mixed-lifetime object graphs unless region boundaries are carefully designed.
- No cycle-specific semantics; reclamation is boundary-driven.

### Complexity
Low-to-medium.

---

### C) Intrusive reference counting in generated C runtime

### What it is
Add `refcount` to `TnObj`, with `retain/release` helpers and recursive destruction.

### Pros
- Deterministic destruction for acyclic data.
- Familiar semantics (already conceptually present in `native_abi`).

### Cons
- High codegen touch surface (every ownership transfer matters).
- Cycles leak unless cycle collector or weak references are added.
- Easy to get wrong with closures and container graphs.

### Complexity
Medium-to-high.

---

### D) Non-moving tracing GC (mark/sweep), optional incremental mode later

### What it is
Keep stable object addresses, add mark bits + root scanning + sweep phase.

### Pros
- Handles cycles naturally.
- Less per-assignment burden than RC.
- Good long-term baseline for long-running runtimes.

### Cons
- Requires robust root enumeration.
- Stop-the-world pauses unless incrementalized.

### Complexity
Medium.

---

### E) Hybrid deferred RC + cycle collector

### What it is
Use RC for fast acyclic cleanup and add cycle detection/collection.

### Pros
- Prompt cleanup for many workloads.
- Can reduce retained garbage in mutation-heavy paths.

### Cons
- Highest implementation complexity.
- More invariants and failure modes.

### Complexity
High.

---

## Decision matrix (practical)

| Approach | Reclaims memory | Cycle-safe | Codegen disruption | Pause profile | Fit for long-running apps |
|---|---:|---:|---:|---:|---:|
| A. Observability | ❌ | n/a | ✅ low | none | ⚠️ limited |
| B. Regions | ✅ (boundary) | n/a | ✅ low/med | none | ⚠️ depends on boundaries |
| C. RC | ✅ (acyclic) | ❌ (without add-ons) | ⚠️ high | low/steady | ⚠️ with cycle risk |
| D. Mark/Sweep | ✅ | ✅ | ⚠️ medium | bursty → smoother if incremental | ✅ strong |
| E. Hybrid RC+Cycle | ✅ | ✅ | ❌ highest | low/steady + cycle bursts | ✅ strong |

---

## Recommended execution plan

### Phase 0 (now): Instrument + benchmark harness
- Add runtime memory counters and optional logging.
- Build deterministic stress fixtures for lists/maps/closures/cycles.
- Capture baseline RSS/high-water metrics.

### Phase 1: Shared substrate for both RC and tracing
- Introduce explicit runtime root registration API in generated runtime.
- Segment heap metadata (kind, mark bit or refcount slot, allocation id).
- Keep behavior identical (still append-only reclaim policy).

### Phase 2A: RC prototype (feature flag)
- Implement retain/release for core container objects and closures.
- Run acyclic leak tests and ownership-fuzz tests.

### Phase 2B: Tracing prototype (feature flag)
- Implement stop-the-world mark/sweep first.
- Validate cycle collection and determinism.

### Phase 3: Bakeoff and default selection
- Compare memory growth, pause behavior, and implementation complexity.
- Choose default strategy (selected: tracing mark/sweep; unset `TONIC_MEMORY_MODE` resolves to `trace`).
- Keep other strategies behind explicit flags (`append_only` rollback and `rc` experiment mode).
- Bakeoff report and CI guardrails:
  - `docs/runtime-memory-bakeoff.md`
  - `scripts/memory-bakeoff.sh --ci`

---

## Scaffolded task set

Tracked scaffold:

- `docs/runtime-memory-task-scaffold.md`

Optional local code-task files (agent task format):

- `.agents/tasks/tonic/runtime-memory/01-memory-observability-and-baselines.code-task.md`
- `.agents/tasks/tonic/runtime-memory/02-memory-substrate-and-root-tracking.code-task.md`
- `.agents/tasks/tonic/runtime-memory/03-reference-counting-prototype.code-task.md`
- `.agents/tasks/tonic/runtime-memory/04-tracing-gc-prototype.code-task.md`
- `.agents/tasks/tonic/runtime-memory/05-memory-strategy-bakeoff-and-default.code-task.md`

---

## External research notes (early 2026)

High-level direction is consistent with current runtime ecosystem practice:

- Lua: tracing GC with incremental/generational modes.
- Wren/Janet: tracing collectors with simpler models.
- QuickJS: RC + cycle handling.
- CPython/PHP: proven RC+cycle patterns and known pitfalls.

For this codebase, a conservative path is:
1) instrument first,
2) build substrate,
3) prototype RC and tracing behind flags,
4) choose based on data.

---

## Sources

- https://www.lua.org/manual/5.5/manual.html
- https://wren.io/embedding/
- https://wren.io/embedding/configuring-the-vm.html
- https://bellard.org/quickjs/
- https://github.com/bdwgc/bdwgc
- https://docs.python.org/3/library/gc.html
- https://docs.python.org/3/c-api/refcounting.html
- https://www.php.net/manual/en/features.gc.collecting-cycles.php
