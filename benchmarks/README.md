# Tonic Benchmark Suite

This suite profiles representative Tonic workloads and can enforce latency thresholds.

## Inputs

- Suite manifest: `benchmarks/suite.toml`
- Runner: `src/bin/benchsuite.rs`

Each workload defines:
- `name`
- `command` (argv passed to `tonic`)
- `mode` (`warm` or `cold`, default `warm`. Cold mode clears `.tonic/cache` before each run)
- `threshold_p50_ms`
- `threshold_p95_ms`

## Run

Build a release binary first:

```bash
cargo build --release
```

Run the suite (JSON printed to stdout + written to file):

```bash
cargo run --bin benchsuite -- --bin target/release/tonic
```

Calibrate thresholds to suggest new limits based on current baseline:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic --calibrate --calibrate-margin-pct 20
```

Custom run count / warmup + markdown output:

```bash
cargo run --bin benchsuite -- \
  --bin target/release/tonic \
  --runs 25 \
  --warmup 5 \
  --json-out benchmarks/summary.json \
  --markdown-out benchmarks/summary.md
```

## Enforce performance requirements

Fail non-zero if any workload exceeds p50/p95 thresholds:

```bash
./scripts/bench-enforce.sh
```

Or manually:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic --enforce
```

This is suitable for CI perf-gate checks.

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

## Mapping to performance requirements

- p50: typical developer experience latency
- p95: tail latency and worst-case responsiveness
- Thresholds in `suite.toml` are explicit contracts; tune downward as perf improves.
