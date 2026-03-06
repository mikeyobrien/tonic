---
name: tonic-observability
description: Use Tonic's local observability harness for debugging compiler/runtime failures, native gates, parity, benchmark, and memory workflows. Helps decide when to enable telemetry and how to inspect bundles.
---

# Tonic Observability

Use this skill when working on Tonic tasks where phase timing, error classification, emitted artifacts, or script-level correlation would help.

## Enable it for

- debugging compiler/runtime failures
- `tonic check`, `tonic run`, or `tonic compile` failures where phase boundaries matter
- native gates, differential/parity, benchmark, release, or memory workflows
- any task where you want emitted artifacts and child runs grouped under one task view

## Keep observability off for

Keep observability off for trivial formatting-only edits, quick read-only inspection, or tiny changes where normal stdout/stderr is already enough. Turn it on once diagnosis starts taking guesses.

## Phase 1 workflow

Phase 1 is local-first and file-first. No extra services are required.

### Single command

```bash
TONIC_OBS_ENABLE=1 cargo run --bin tonic -- check path/to/file.tn
```

Optional overrides:

- `TONIC_OBS_DIR=/tmp/tonic-obs`
- `TONIC_OBS_TASK_ID=<task-id>` to correlate related runs
- `TONIC_OBS_RUN_ID=<run-id>` or `TONIC_OBS_PARENT_RUN_ID=<run-id>` when a wrapper already manages identities

### Script workflow

```bash
TONIC_OBS_ENABLE=1 ./scripts/native-gates.sh
```

Top-level scripts create a root run plus child step runs under the same task.

## Where to inspect bundles

Default root: `.tonic/observability/`

Important files:

- `latest.json` — pointer to the most recent run summary
- `runs/<run-id>/summary.json` — primary contract: command, status, phases, artifacts, normalized error
- `runs/<run-id>/events.jsonl` — append-only event stream
- `runs/<run-id>/artifacts.json` — emitted artifact manifest
- `tasks/<task-id>/runs.jsonl` — compact index for correlated multi-run workflows

## How to use the data

1. Open `latest.json` to find the last summary quickly.
2. Read `summary.json` first.
   - `status`, `exit_code`
   - `phases[]`
   - `error.kind`, `error.phase`, `error.source`
   - `artifacts.emitted[]`
3. If the run came from a script, open `tasks/<task-id>/runs.jsonl` to find child runs.
4. Use `events.jsonl` when you need step-by-step timing or script step boundaries.

## Good defaults

- Use telemetry for native gates before chasing failures across multiple scripts.
- Use telemetry for benchmark and memory work when artifact paths matter.
- Leave legacy knobs alone: `TONIC_PROFILE_*`, `TONIC_DEBUG_*`, and `TONIC_MEMORY_*` still work and are reflected in the run summary.

## Phase 2

Phase 2 is optional future work: OTLP export and a disposable local UI stack. Do not assume it exists. Start with the local bundle every time.

For fuller operator-facing details, read `docs/observability.md`.
