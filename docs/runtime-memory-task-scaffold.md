# Runtime Memory Task Scaffold

This is a tracked scaffold of the memory roadmap work in `docs/memory-management-roadmap.md`.

## Task 01 — Memory observability + baselines
- Add allocation/high-water counters in generated runtime and native ABI heap.
- Add opt-in diagnostics mode (`TONIC_MEMORY_STATS=1`).
- Add stress fixtures and baseline harness output.
- Exit criteria: deterministic counters, no semantic regressions.

## Task 02 — Memory substrate + root tracking
- Add shared object metadata slot(s) needed for future collectors.
- Add root registration API around call/closure/host boundaries.
- Keep behavior append-only in this phase.
- Exit criteria: no output diffs; root tracking covered by tests.

## Task 03 — RC prototype (feature flagged)
- Add object refcount with retain/release hooks.
- Apply ownership transitions across containers/calls/closure captures.
- Gate RC behind `TONIC_MEMORY_MODE=rc` while default remains append-only.
- Document cycle caveat (`cycle_collection=off`) and add acyclic leak tests.
- Exit criteria: acyclic workloads reclaim, parity tests remain green.

## Task 04 — Tracing GC prototype (feature flagged)
- Add mark metadata, root traversal, stop-the-world mark/sweep trigger.
- Gate tracing mode behind `TONIC_MEMORY_MODE=trace` (default remains append-only).
- Emit deterministic tracing stats (`memory_mode=trace`, `cycle_collection=mark_sweep`, `gc_collections_total`).
- Validate cyclic reclamation with dedicated fixtures.
- Keep deterministic diagnostics.
- Exit criteria: cycle fixtures reclaim; semantic tests green.

## Task 05 — Bakeoff + default selection
- Compare baseline vs RC vs tracing on startup, throughput, RSS growth, pause profile.
- Choose default strategy and document rationale + rollback path.
- Add CI guardrails for regressions.
- Implementation notes:
  - Repro harness: `scripts/memory-bakeoff.sh`
  - CI guardrail mode: `scripts/memory-bakeoff.sh --ci`
  - Report: `docs/runtime-memory-bakeoff.md`
  - Selected default: tracing mark/sweep (unset `TONIC_MEMORY_MODE` resolves to `trace`)
  - Rollback: `TONIC_MEMORY_MODE=append_only`
- Exit criteria: reproducible report and explicit default strategy.
