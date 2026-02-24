---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Define Native-Compiler Performance Targets and Baseline Gap

## Description
Create a hard, measurable performance contract for the native compiler effort so work can be prioritized by impact and validated against Rust/Go-class expectations.

## Background
Current Tonic execution is interpreter-based. Before backend implementation, we need explicit workload-level targets (cold start, steady-state throughput, memory, binary size) and comparable baselines across Tonic interpreter, Rust, and Go implementations.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `benchmarks/README.md`
- `benchmarks/suite.toml`
- `.agents/tasks/2026-02-24-cross-language-benchmarking/01-cross-language-competitive-benchmarking.code-task.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define explicit SLOs for native mode (startup, p50/p95 runtime, RSS, artifact size, compile latency).
2. Add benchmark workloads that represent CLI scripting reality (parse/check/run, module load, JSON/file/subprocess/control-flow heavy).
3. Produce baseline data for: interpreter mode, Rust equivalent, Go equivalent.
4. Introduce a weighted scoring model and pass/fail thresholds for “Rust/Go competitive.”
5. Persist benchmark metadata (hardware, OS, toolchain versions) for reproducibility.

## Dependencies
- Existing benchsuite runner and manifest format.
- Cross-language benchmark task scaffolding.
- Existing parity and reliability fixtures.

## Implementation Approach
1. Extend benchmark manifest schema to support multi-target baselines and weighted scoring.
2. Add Rust/Go reference workload implementations and runner wiring.
3. Generate initial benchmark report and lock target thresholds into repo config/docs.

## Acceptance Criteria

1. **Performance Contract Is Explicit**
   - Given the benchmark config in repository
   - When a contributor inspects native-compiler requirements
   - Then workload-level thresholds and weighted pass criteria are documented and machine-readable.

2. **Gap Baseline Exists**
   - Given benchmark runner execution
   - When the suite is run on a clean machine profile
   - Then reports include interpreter vs Rust vs Go absolute and relative metrics.

3. **Regression Signal Is Enforceable**
   - Given CI or local enforce mode
   - When measured metrics exceed configured thresholds
   - Then the benchmark gate fails with deterministic diagnostics.

4. **Unit/Integration Tests Cover Schema and Scoring**
   - Given benchmark config parsing and scoring logic
   - When running `cargo test`
   - Then tests validate schema compatibility, weighted scoring math, and failure behavior.

## Metadata
- **Complexity**: Medium
- **Labels**: Benchmarking, Performance, Planning, CI-Gates
- **Required Skills**: Benchmark design, statistical reporting, Rust test design
