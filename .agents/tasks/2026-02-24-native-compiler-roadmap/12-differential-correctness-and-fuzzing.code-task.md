---
status: completed
HEARTBEAT_TASK_STATUS: done
started: 2026-02-24
completed: 2026-02-24
---

# Task: Add Differential Correctness Harness and Fuzzing Across Backends

## Description
Build a correctness system that continuously compares interpreter and native backend behavior across curated fixtures and generated programs.

## Background
Native backends often diverge subtly. Differential testing is mandatory to avoid semantic drift while optimization and runtime changes accelerate.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `examples/parity/catalog.toml`
- `tests/run_parity_examples.rs`
- `tests/run_translated_fixtures_smoke.rs`
- `research/reliability-regression-matrix.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Add a differential runner that executes fixtures in interpreter and native modes and compares stdout/stderr/exit behavior.
2. Add property-based or fuzz-generated program tests for safe expression/control-flow subsets.
3. Add minimization and artifact capture for mismatches.
4. Integrate differential checks into CI gates for backend changes.
5. Provide developer workflow docs for reproducing and triaging divergences.

## Dependencies
- Native CLI integration (task 11).
- Existing parity/fixture infrastructure.
- Reliability test conventions.

## Implementation Approach
1. Build a shared comparison harness module reusable in tests and scripts.
2. Add deterministic fixture comparison tests over parity catalog subsets.
3. Integrate fuzz/property tests with reproducible seeds and failure snapshots.

## Acceptance Criteria

1. **Interpreter vs Native Differential Runner Works**
   - Given selected fixture sets
   - When differential runner executes both backends
   - Then mismatches are detected and reported with reproducible artifacts.

2. **Fuzzing Detects Divergence and Is Reproducible**
   - Given randomized or property-based test inputs
   - When divergences occur
   - Then failing seeds/cases are persisted for deterministic replay.

3. **CI Gate Blocks Semantic Regressions**
   - Given backend PRs affecting runtime/codegen
   - When differential checks fail
   - Then CI reports deterministic failures and blocks merge.

4. **Unit/Integration Tests Cover Differential Tooling**
   - Given differential harness code
   - When running `cargo test`
   - Then tests cover comparison logic, mismatch reporting, and replay behavior.

## Metadata
- **Complexity**: High
- **Labels**: Correctness, Differential-Testing, Fuzzing, CI
- **Required Skills**: Test harness design, property testing, regression triage
