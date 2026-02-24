# Tonic Benchmark Suite

This suite profiles representative Tonic workloads and can enforce latency/performance contracts.

## Inputs

- Legacy suite manifest: `benchmarks/suite.toml`
- Native compiler contract suite: `benchmarks/native-compiler-suite.toml`
- Rust/Go baseline data: `benchmarks/native-compiler-baselines.json`
- Runner: `src/bin/benchsuite.rs`

Each workload defines:
- `name`
- `command` (argv passed to `tonic`)
- `mode` (`warm` or `cold`, default `warm`; cold mode clears `.tonic/cache`)
- `threshold_p50_ms`
- `threshold_p95_ms`
- optional `threshold_rss_kb`
- optional `weight`
- optional `category`

Native compiler suites can also define `[performance_contract]` with:
- native SLOs (`startup`, `runtime`, `rss`, `artifact_size`, `compile_latency`)
- weighted scoring (`metric_weights`, `pass_threshold`)
- reference baseline targets (`rust`, `go`) + `relative_budget_pct`

## Run

Build a release binary first:

```bash
cargo build --release
```

Run the legacy suite:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic
```

Run the native compiler contract suite (includes weighted Rust/Go comparisons):

```bash
cargo run --bin benchsuite -- \
  --bin target/release/tonic \
  --manifest benchmarks/native-compiler-suite.toml \
  --target-name interpreter \
  --compile-latency-ms 2600 \
  --json-out benchmarks/native-compiler-summary.json \
  --markdown-out benchmarks/native-compiler-summary.md
```

Calibrate workload thresholds:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic --calibrate --calibrate-margin-pct 20
```

## Enforce performance requirements

Fail non-zero if any configured threshold or contract gate fails:

```bash
./scripts/bench-enforce.sh
```

Or manually:

```bash
cargo run --bin benchsuite -- \
  --bin target/release/tonic \
  --manifest benchmarks/native-compiler-suite.toml \
  --target-name interpreter \
  --compile-latency-ms 2600 \
  --enforce
```

In contract mode, enforce checks:
- absolute workload thresholds (`p50`, `p95`, optional `rss`)
- weighted overall competitiveness score
- native SLO thresholds
- deterministic failure reasons in JSON/Markdown reports

## Reproducibility metadata

Runner output now persists host metadata in every report:
- OS/arch
- kernel
- CPU model
- rustc version
- go version
- capture timestamp

Reference baseline metadata is also included in contract report output.

## Profiling hotspots

If a workload regresses, profile the specific command:

```bash
cargo flamegraph --release --bin tonic -- run examples/parity/06-control-flow/for_multi_generator.tn
```

Or with `perf`:

```bash
perf record -g target/release/tonic run examples/parity/06-control-flow/for_multi_generator.tn
perf report
```

For built-in phase timing (frontend/codegen/runtime), set a profile sink:

```bash
TONIC_PROFILE_OUT=benchmarks/phase-profile.jsonl \
  target/release/tonic compile examples/parity/06-control-flow/for_multi_generator.tn --backend llvm --emit object
```

Each command appends one JSON line with `command`, `total_ms`, and per-phase `elapsed_ms` so regressions can be localized before full flamegraph runs.
