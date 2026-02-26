# Tonic

> **Elixir-inspired language design. Rust implementation. Native binaries.**
>
> Tonic is an alpha-stage language core for developers who want expressive functional syntax *and* a practical compiler/runtime workflow.

## Why Tonic?

Tonic is opinionated about developer experience:

- **Readable, composable syntax** — modules, clauses, pattern matching, guards, and functional control flow.
- **Fast inner loop** — run code immediately with `tonic run`.
- **Native output path** — compile to a runnable executable with `tonic compile`.
- **Serious engineering workflow** — parity tracking, differential tests, and native gate scripts are built into the repo.

If you like Elixir-style code shape but want to explore language/runtime implementation in Rust, Tonic is built for that.

## 60-second demo

### 1) Run a program via interpreter

```bash
cargo run --bin tonic -- run examples/parity/02-operators/arithmetic_basic.tn
```

Output:

```text
{3, {2, {8, 5}}}
```

### 2) Compile the same program to a native executable

```bash
cargo run --bin tonic -- compile examples/parity/02-operators/arithmetic_basic.tn --out ./.tonic/build/arithmetic_basic
./.tonic/build/arithmetic_basic
```

Same output, different execution path.

## Project status

- **Version:** `0.1.0-alpha.1`
- **Stability:** alpha (interfaces and behavior may still evolve)
- **Scope:** language syntax/runtime parity work and native backend iteration
- **Out of scope:** BEAM/OTP runtime model (processes, supervisors, distribution, hot upgrade lifecycle)

For detailed parity coverage and planned gaps, see [PARITY.md](PARITY.md).

## What you get today

- Frontend pipeline: lexer → parser → resolver → type inference
- IR + MIR lowering
- Interpreter runtime (`tonic run`)
- Native compile flow with C/LLVM sidecars (`tonic compile`)
- Multi-file project entry via `tonic.toml`
- `.tn` test runner with text/JSON output (`tonic test`)
- Formatting and static checking (`tonic fmt`, `tonic check`)
- Dependency lock/sync workflows (`tonic deps`)

## Quickstart

### Prerequisites

- Rust toolchain (`cargo`, `rustc`)
- C compiler in `PATH` (`clang`, `gcc`, or `cc`) for native compile/link
- `git`
- `python3` (used by some scripts)

### Build

```bash
cargo build --bin tonic
```

> This repository has multiple binaries, so use `--bin tonic` with `cargo run`.

### Run checks

```bash
cargo test
cargo run --bin tonic -- fmt examples --check
```

## Minimal language example

```tn
defmodule Demo do
  def run() do
    with {:ok, v1} <- {:ok, 10},
         {:ok, v2} <- {:ok, 20} do
      v1 + v2
    end
  end
end
```

Run it:

```bash
cargo run --bin tonic -- run path/to/file.tn
```

## CLI cheat sheet

| Command | Purpose | Example |
|---|---|---|
| `tonic run <path>` | Execute a file or project (`tonic.toml`) | `cargo run --bin tonic -- run examples/parity/07-modules/project_multifile_pipeline` |
| `tonic check <path> [--dump-tokens\|--dump-ast\|--dump-ir\|--dump-mir]` | Parse/type-check and optionally dump internals | `cargo run --bin tonic -- check examples/parity/01-literals/atom_expression.tn --dump-ir` |
| `tonic test <path> [--format <text\|json>]` | Run discovered `.tn` tests | `cargo run --bin tonic -- test examples/parity --format json` |
| `tonic fmt <path> [--check]` | Format source files or verify formatting | `cargo run --bin tonic -- fmt examples --check` |
| `tonic compile <path> [--out <artifact-path>]` | Produce native executable + sidecars | `cargo run --bin tonic -- compile examples/parity/02-operators/arithmetic_basic.tn` |
| `tonic deps <sync\|fetch\|lock>` | Sync/fetch/lock dependencies for a `tonic.toml` project | `tonic deps lock` |
| `tonic verify run <slice-id> [--mode <auto\|mixed\|manual>]` | Run acceptance verification flow | `tonic verify run step-01 --mode auto` |

`tonic cache` currently exists as a placeholder command surface.

## Native compile artifacts

By default, compile outputs are written to `.tonic/build/<stem>`:

- Executable: `<stem>`
- LLVM IR sidecar: `<stem>.ll`
- C source sidecar: `<stem>.c`
- Tonic IR sidecar: `<stem>.tir.json`
- Native artifact manifest: `<stem>.tnx.json`

## Architecture at a glance

```mermaid
graph TD
    CLI[tonic CLI]
    CLI --> LOAD[load source/manifest]
    LOAD --> LEX[lexer]
    LEX --> PARSE[parser]
    PARSE --> RESOLVE[resolver]
    RESOLVE --> TYPE[type inference]
    TYPE --> IR[IR lowering]

    IR --> INTERP[interpreter runtime]
    IR --> MIR[MIR lowering]
    MIR --> OPT[optimization]
    OPT --> CBACK[C backend]
    OPT --> LLVM[LLVM backend]
    CBACK --> LINK[system compiler/linker]
    LINK --> EXE[native executable]
```

## Engineering quality gates

Tonic ships with high-signal validation workflows:

```bash
./scripts/differential-enforce.sh
./scripts/llvm-catalog-parity-enforce.sh
./scripts/native-gates.sh
```

Release-readiness gate:

```bash
./scripts/release-alpha-readiness.sh --version X.Y.Z-alpha.N
```

Benchmark docs and manifests:

- [benchmarks/README.md](benchmarks/README.md)
- [benchmarks/native-compiler-suite.toml](benchmarks/native-compiler-suite.toml)
- [benchmarks/native-compiled-suite.toml](benchmarks/native-compiled-suite.toml)

## Diagnostics and profiling hooks

- `TONIC_DEBUG_CACHE=1` — cache hit/miss traces
- `TONIC_DEBUG_MODULE_LOADS=1` — module-load traces
- `TONIC_DEBUG_TYPES=1` — type-signature summaries
- `TONIC_PROFILE_STDERR=1` — per-phase timings on stderr
- `TONIC_PROFILE_OUT=<path>` — JSONL timing output
- `TONIC_MEMORY_MODE=<append_only|rc|trace>` + `TONIC_MEMORY_STATS=1` — memory diagnostics

## Repository layout

- `src/` — compiler/runtime/backends
- `tests/` — integration + contract tests
- `examples/` — parity fixtures and app examples
- `benchmarks/` — benchmark manifests and baselines
- `scripts/` — gate, benchmark, and release scripts
- `docs/` — focused technical docs
- `PARITY.md` — syntax parity checklist and priorities

## Documentation map

- [PARITY.md](PARITY.md)
- [docs/native-runtime.md](docs/native-runtime.md)
- [docs/runtime-abi.md](docs/runtime-abi.md)
- [docs/differential-testing.md](docs/differential-testing.md)
- [docs/native-regression-policy.md](docs/native-regression-policy.md)
- [docs/release-checklist.md](docs/release-checklist.md)
- [CHANGELOG.md](CHANGELOG.md)

## Contributing

Contributions are welcome.

Suggested preflight before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

For parity-sensitive/runtime-sensitive changes, also run:

```bash
./scripts/native-gates.sh
```

Repository-specific working notes are in [AGENTS.md](AGENTS.md).

## Roadmap (near term)

See [PARITY.md](PARITY.md) for full tracking. Key near-term gaps:

- Numeric literal parity (hex/octal/binary, numeric separators, char literals)
- Operator parity (`===`, `!==`, `div`, `rem`, `not in`, stepped ranges, bitwise family)
- Bitstring/binary pattern parity
- Additional compile-time/module form parity
- `tonic docs` command surface

## License

This project is licensed under the [MIT License](LICENSE).
