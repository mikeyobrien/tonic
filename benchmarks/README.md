# Tonic Benchmarking Suite

This directory contains the benchmarking suite for the Tonic language. It's designed to profile key workloads and enforce performance thresholds.

## Workloads
The suite loads its configuration from `suite.toml`. Each workload defines a Tonic command (e.g., `check`, `run`) and the target file.

## Usage

### Run the Benchmark Suite

To execute the benchmarking suite and see performance results:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic
```

The suite performs warmup runs followed by measured runs, computing the p50 and p95 latency for each workload. It outputs the results to stdout and saves a JSON report at `benchmarks/summary.json`.

### Enforce Mode (CI/CD)

To run the suite in "enforce" mode, which will fail with a non-zero exit code if any workload exceeds its defined p50 or p95 thresholds:

```bash
cargo run --bin benchsuite -- --bin target/release/tonic --enforce
```

## Profiling Hotspots

If a workload is failing its performance thresholds or you want to analyze its execution, use `perf` and `flamegraph`:

1.  **Install tools:**
    ```bash
    cargo install flamegraph
    ```

2.  **Generate a Flamegraph:**
    ```bash
    cargo flamegraph --bin tonic -- run examples/parity/06-control-flow/for_multi_generator.tn
    ```
    This will generate a `flamegraph.svg` file that visualizes CPU usage across the Tonic codebase during the execution of that specific workload.

3.  **Use Perf Directly:**
    ```bash
    perf record -g target/release/tonic run examples/parity/06-control-flow/for_multi_generator.tn
    perf report
    ```

## Performance Requirements

The benchmarking suite maps directly to Tonic's performance requirements:
- Ensure the core parsing/checking pipelines stay consistently fast for typical files.
- Prevent regressions in bytecode generation and execution speed for loops (`for`), exception handling (`try`/`rescue`), and project evaluation.
- The thresholds in `suite.toml` are considered the upper bound of acceptable latency for an average developer machine.
