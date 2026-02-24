---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Enforce Rust/Go Competitive Gates in CI and Release Process

## Description
Finalize the native backend program by adding CI/release gates that continuously verify Rust/Go competitiveness, semantic stability, and operational reliability.

## Background
Without automated gates, performance and correctness gains decay quickly. This final task converts roadmap outcomes into sustained engineering constraints.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `benchmarks/README.md`
- `scripts/bench-enforce.sh`
- `research/reliability-regression-matrix.md`
- `.githooks/pre-push`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Add CI jobs for native backend build, differential correctness, and perf enforcement.
2. Add Rust/Go comparative benchmark publishing artifacts for each candidate release.
3. Define regression policy (allowed variance, quarantine flow, rollback criteria).
4. Add release checklist requiring green native gates before version/tag cut.
5. Ensure local developer commands can reproduce CI gate outcomes.

## Dependencies
- Benchmark and scoring contract (task 01).
- Differential harness (task 12).
- Optimized native backend (task 13).

## Implementation Approach
1. Add CI workflow(s) for backend matrix and benchmark artifact publishing.
2. Add deterministic gate scripts/config for local and CI usage.
3. Update contributor/release documentation with explicit native-goals policy.

## Acceptance Criteria

1. **CI Enforces Native Correctness and Performance**
   - Given backend-related changes
   - When CI pipelines run
   - Then failures in differential or performance gates block merge.

2. **Rust/Go Comparison Artifacts Are Produced**
   - Given benchmark CI execution
   - When jobs complete
   - Then artifacts include Tonic-native vs Rust vs Go comparisons and metadata.

3. **Release Process Requires Green Native Gates**
   - Given a release candidate
   - When release checklist is applied
   - Then native correctness/performance gates are mandatory and auditable.

4. **Unit/Integration Tests Cover Gate Tooling**
   - Given CI/local gate scripts and parsers
   - When running `cargo test` and script smoke tests
   - Then gate tooling behavior is validated for pass/fail scenarios.

## Metadata
- **Complexity**: Medium
- **Labels**: CI, Release, Performance-Gates, Reliability
- **Required Skills**: CI pipeline design, release engineering, benchmark governance
