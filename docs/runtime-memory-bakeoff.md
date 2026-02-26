# Runtime memory bakeoff (Task 05)

This report records the Task 05 baseline-vs-RC-vs-trace comparison and the default memory-mode decision.

## Reproduction

```bash
./scripts/memory-bakeoff.sh
```

Artifacts:

- Raw runs: `.tonic/memory-bakeoff/raw.tsv`
- Aggregated summary: `.tonic/memory-bakeoff/summary.tsv`
- Markdown summary: `.tonic/memory-bakeoff/summary.md`

CI guardrails run the same harness in assertion mode:

```bash
./scripts/memory-bakeoff.sh --ci
```

## Results (baseline vs RC vs trace)

Baseline is `TONIC_MEMORY_MODE=append_only`.

| Scenario | Metric | Baseline (append_only) | RC (`TONIC_MEMORY_MODE=rc`) | Trace (`TONIC_MEMORY_MODE=trace`) |
| --- | --- | ---: | ---: | ---: |
| startup | median elapsed (ms) | 2.669 | 3.290 | 2.438 |
| startup | median RSS (KiB) | 2952 | 2864 | 2856 |
| throughput | median elapsed (ms) | 3.005 | 2.909 | 2.991 |
| throughput | median RSS (KiB) | 2944 | 2888 | 2872 |
| cycle_churn | median reclaims_total | 0 | 1 | 4801 |
| cycle_churn | median heap_live_slots | 4801 | 4800 | 0 |
| cycle_churn | median gc_collections_total | 0 | 0 | 2 |

Notes:

- RC reclaims acyclic objects but still retains cyclic structures in this fixture.
- Tracing consistently reclaims cycle churn objects and drops live slots to 0 at process end.
- Startup and throughput medians stay within a narrow band across modes in this harness.

## Default strategy decision

Selected default: **trace mark/sweep**.

Rationale:

1. It is the only mode that reclaims cyclic graphs deterministically in the current runtime.
2. Measured startup/throughput impact stays within acceptable range for the benchmark scenarios.
3. Ownership complexity is lower than making RC fully cycle-safe.

## Rollback path

Immediate rollback path is explicit and reversible:

- `TONIC_MEMORY_MODE=append_only` — restores pre-collector append-only behavior.
- `TONIC_MEMORY_MODE=rc` — keeps RC prototype available for experiments.

Default-mode guardrail in CI also verifies that `TONIC_MEMORY_MODE` unset resolves to trace.
