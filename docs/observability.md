# Observability

Tonic ships a local-first observability harness for debugging non-trivial CLI and script workflows.

It is intentionally boring:

- default-off
- file-first
- per-worktree
- fail-open
- no network dependency required

If observability cannot write its bundle, Tonic still preserves the real command or script exit behavior and prints a warning instead of inventing a new failure.

## When to use it

Enable observability when you need more than plain stdout/stderr:

- debugging compiler/runtime failures
- understanding which phase failed during `run`, `compile`, `check`, `test`, `fmt`, `verify`, or `deps`
- correlating multi-step script workflows like `native-gates`, differential/parity runs, release readiness, benchmark enforcement, or memory bakeoffs
- finding emitted artifacts without guessing which script wrote them

Leave it off for trivial edits or read-only inspection when normal output is already enough.

## Agent skill

A project-local Pi skill lives at `.agents/skills/tonic-observability/SKILL.md`.
Use it when an agent is doing debugging, parity, benchmark, memory, or gate work and needs a reminder about when to enable telemetry and where to inspect bundles.

## Phase 1

Phase 1 is what exists today.

### Enable telemetry

For CLI commands:

```bash
TONIC_OBS_ENABLE=1 cargo run --bin tonic -- check examples/parity/01-literals/atom_expression.tn
```

For repo scripts:

```bash
TONIC_OBS_ENABLE=1 ./scripts/native-gates.sh
```

## Environment variables

### New observability vars

- `TONIC_OBS_ENABLE=1` — enable local bundle writing
- `TONIC_OBS_DIR=<path>` — override the output root; default is `.tonic/observability/` under the current worktree
- `TONIC_OBS_RUN_ID=<id>` — provide a run id explicitly
- `TONIC_OBS_TASK_ID=<id>` — correlate related runs under one task id
- `TONIC_OBS_PARENT_RUN_ID=<id>` — link a child run to a parent run

### Existing signals that stay valid

Observability layers on top of the existing env surface instead of replacing it:

- `TONIC_PROFILE_STDERR`
- `TONIC_PROFILE_OUT`
- `TONIC_DEBUG_CACHE`
- `TONIC_DEBUG_MODULE_LOADS`
- `TONIC_DEBUG_TYPES`
- `TONIC_MEMORY_MODE`
- `TONIC_MEMORY_STATS`

Those signals are reflected back in `summary.json` under `legacy_signals` when observability is enabled.

## Bundle layout

Default output root:

```text
.tonic/observability/
  latest.json
  runs/<run-id>/summary.json
  runs/<run-id>/events.jsonl
  runs/<run-id>/artifacts.json
  tasks/<task-id>/runs.jsonl
```

### What each file is for

- `latest.json`
  - quick pointer to the latest run and its `summary.json`
- `runs/<run-id>/summary.json`
  - the main file to open first
  - includes command identity, status, exit code, phases, artifacts, normalized error data, and legacy signal flags
- `runs/<run-id>/events.jsonl`
  - append-only event stream
  - includes `run.started`, `phase.finished`, `artifact.written`, `error.reported`, `step.started`, `step.finished`, and `run.finished`
- `runs/<run-id>/artifacts.json`
  - compact artifact manifest
- `tasks/<task-id>/runs.jsonl`
  - compact index for correlated script/task workflows
  - useful for `native-gates` and other wrapper-driven runs

## Common workflows

### 1. Debug a failing `check`

```bash
TONIC_OBS_ENABLE=1 cargo run --bin tonic -- check path/to/file.tn
```

Then inspect:

1. `.tonic/observability/latest.json`
2. `runs/<run-id>/summary.json`
3. `error.kind`, `error.phase`, and `error.source`
4. `phases[]` to see how far the run progressed

### 2. Inspect compile outputs without guessing

```bash
TONIC_OBS_ENABLE=1 cargo run --bin tonic -- compile path/to/file.tn --out ./.tonic/build/demo
```

Open `runs/<run-id>/artifacts.json` or `summary.json` and check `artifacts.emitted[]` for the executable and sidecars.

### 3. Correlate a script workflow

```bash
TONIC_OBS_ENABLE=1 ./scripts/native-gates.sh
```

Start with the root `summary.json`, then open `tasks/<task-id>/runs.jsonl` to find child runs for steps such as formatting, clippy, tests, parity, benchmark enforcement, and memory bakeoff.

## How to read a bundle quickly

1. Open `latest.json`.
2. Read `summary.json`.
3. If `status` is `error`, look at:
   - `error.kind`
   - `error.phase`
   - `error.source`
4. If the workflow emitted files, inspect `artifacts.emitted[]` or `artifacts.json`.
5. If the run came from a script, pivot to `tasks/<task-id>/runs.jsonl` and then into child run summaries.
6. Use `events.jsonl` only when you need the full sequence of phase or step events.

## Phase 2

Phase 2 is not part of the required local setup today.

Reserved future ideas:

- optional OTLP export
- optional disposable local UI stack
- optional Grafana/Tempo-style exploration on top of the same run/task model

That work is intentionally optional and later. The Phase 1 contract is the local bundle under `.tonic/observability/`.
