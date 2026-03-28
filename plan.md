# Plan

## Status
The active backpressure experiment is finished and kept.

- Primary metric improved from **80** uncovered eligible fixtures to **0**.
- Validation evidence is already collected and recorded in `.miniloop/autoresearch.md` and `.miniloop/progress.md`.
- There is no active implementation slice.

## Default next action
The strategist should emit **`task.complete`**.

If the latest routing event is already `task.complete`, do not treat any generic topology suggestions as new work. Exit cleanly unless a new exclusion-burndown request appears.

## Only if explicit follow-up work is requested
Start a new experiment only for one of these exclusion families:
1. Native `tn_runtime_for` support gaps (6 fixtures)
2. Multi-clause anonymous-function capture lowering gap (1 fixture)
3. Native runtime diagnostic text parity gap (1 fixture)

If a follow-up experiment is opened:
- choose exactly one family
- define a new primary metric before changing code
- keep the change scoped and reversible
- collect both raw measurements and correctness evidence before evaluation

## Not active
- No unrelated CLI/install work
- No new broad parity initiative without a fresh metric and hypothesis

## Ready-for-completion checklist
- [x] Goal of this loop slice is met
- [x] Evidence for keep is documented
- [x] Remaining gaps are explicit and named
- [x] Next role guidance points to completion by default