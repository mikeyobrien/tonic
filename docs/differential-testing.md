# Differential Correctness and Fuzzing Workflow

This project includes a differential gate that compares interpreter behavior (`tonic run <source>`) against native AOT behavior (`tonic compile --backend llvm --emit executable` + `tonic run <.tnx.json>`).

## Run the gate

```bash
cargo test --test differential_backends -- --nocapture
```

This test target runs two suites:

1. **Parity subset differential** over curated active fixtures from `examples/parity/catalog.toml`.
2. **Seeded generator differential** over deterministic generated programs.

## LLVM catalog parity gate (enforced)

Run the full catalog parity harness in enforce mode:

```bash
./scripts/llvm-catalog-parity-enforce.sh
```

This command fails non-zero on any compile/runtime mismatch and writes reports to:

- `.tonic/parity/llvm-catalog-parity.json`
- `.tonic/parity/llvm-catalog-parity.md`

CI runs the same script in `native-gates` and uploads `.tonic/parity/` for triage.

## Reproducible fuzz seeds

Run a single seed:

```bash
TONIC_DIFF_SEED=17 cargo test --test differential_backends -- --nocapture
```

Run a larger seed count:

```bash
TONIC_DIFF_SEEDS=256 cargo test --test differential_backends -- --nocapture
```

Defaults: `TONIC_DIFF_SEEDS=32` and seed range `[0, N)`.

## Divergence triage artifacts

On mismatch, the harness writes a replay bundle under:

- `<temp-root>/differential-artifacts/<label>/program.tn`
- `<temp-root>/differential-artifacts/<label>/program.min.tn`
- `<temp-root>/differential-artifacts/<label>/mismatch.json`

`mismatch.json` includes:

- fixture/seed identity
- mismatch reason (`exit code mismatch`, `stdout mismatch`, `stderr mismatch`, or native compile failure)
- interpreter and native command outputs
- replay command lines

## Manual replay recipe

Given a failing fixture `<path>`:

```bash
tonic run <path>
tonic compile <path> --backend llvm --emit executable
tonic run .tonic/build/<stem>.tnx.json
```

Compare exit code, stdout, and stderr exactly.
