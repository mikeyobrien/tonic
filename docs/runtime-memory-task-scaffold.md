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
- Document cycle caveat and add acyclic leak tests.
- Exit criteria: acyclic workloads reclaim, parity tests remain green.

## Task 04 — Tracing GC prototype (feature flagged)
- Add mark metadata, root traversal, stop-the-world mark/sweep trigger.
- Validate cyclic reclamation with dedicated fixtures.
- Keep deterministic diagnostics.
- Exit criteria: cycle fixtures reclaim; semantic tests green.

## Task 05 — Bakeoff + default selection
- Compare baseline vs RC vs tracing on startup, throughput, RSS growth, pause profile.
- Choose default strategy and document rationale + rollback path.
- Add CI guardrails for regressions.
- Exit criteria: reproducible report and explicit default strategy.
