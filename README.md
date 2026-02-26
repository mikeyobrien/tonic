# Tonic

> Elixir-inspired language core in Rust (alpha).
>
> Tonic supports two execution paths:
> - `tonic run` for interpreter execution
> - `tonic compile` for native executable output

## Why Tonic

Tonic is a language-core project focused on **syntax ergonomics + fast feedback loops** for compiler/runtime development.

The design intentionally leans on Elixir-style constructs (pattern matching, clauses, immutable data flow) because they are readable, composable, and practical for LLM-assisted coding workflows.

## Project status

- **Version:** `0.1.0-alpha.1`
- **Stability:** alpha (active development, interfaces may change)
- **Primary focus:** language syntax/runtime parity workflows and native backend iteration
- **Out of scope:** BEAM/OTP runtime behavior (processes, supervisors, distribution, hot reload lifecycle)

For tracked parity coverage and gaps, see [PARITY.md](PARITY.md).

## Features (today)

- Frontend pipeline: lexer → parser → resolver → type inference
- IR + MIR lowering pipeline
- Interpreter runtime execution (`tonic run`)
- Native compile path with C/LLVM sidecars (`tonic compile`)
- Project-level `tonic.toml` support for multi-file projects
- `tonic test` runner with text/JSON output
- Formatting and static-check flows (`fmt`, `check`)
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

> Note: this repository contains multiple binaries. Use `--bin tonic` with `cargo run`.

### Run your first program

```bash
cargo run --bin tonic -- run examples/parity/02-operators/arithmetic_basic.tn
```

Expected output:

```text
{3, {2, {8, 5}}}
```

### Check + dump AST

```bash
cargo run --bin tonic -- check examples/parity/06-control-flow/for_multi_generator.tn --dump-ast
```

### Compile to a native executable

```bash
cargo run --bin tonic -- compile examples/parity/02-operators/arithmetic_basic.tn --out ./.tonic/build/arithmetic_basic
./.tonic/build/arithmetic_basic
```

### Run tests

```bash
cargo test
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

## CLI reference

| Command | Purpose | Example |
|---|---|---|
| `tonic run <path>` | Execute a file or project (`tonic.toml`) | `cargo run --bin tonic -- run examples/parity/07-modules/project_multifile_pipeline` |
| `tonic check <path> [--dump-tokens\|--dump-ast\|--dump-ir\|--dump-mir]` | Parse/type-check and optionally dump internals | `cargo run --bin tonic -- check examples/parity/01-literals/atom_expression.tn --dump-ir` |
| `tonic test <path> [--format <text\|json>]` | Run discovered `.tn` tests | `cargo run --bin tonic -- test examples/parity --format json` |
| `tonic fmt <path> [--check]` | Format source files or verify formatting | `cargo run --bin tonic -- fmt examples --check` |
| `tonic compile <path> [--out <artifact-path>]` | Produce native executable + sidecars | `cargo run --bin tonic -- compile examples/parity/02-operators/arithmetic_basic.tn` |
| `tonic deps <sync\|fetch\|lock>` | Sync/fetch/lock dependencies for a `tonic.toml` project | `tonic deps lock` |
| `tonic verify run <slice-id> [--mode <auto\|mixed\|manual>]` | Run acceptance verification workflow | `tonic verify run step-01 --mode auto` |

`tonic cache` exists as a placeholder command surface and is not a complete cache-management UX yet.

## Native compile artifacts

By default, compile outputs are written under `.tonic/build/<stem>`:

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

## Repository layout

- `src/` — compiler/runtime/backends
- `tests/` — integration + contract tests
- `examples/` — parity fixtures and app examples
- `benchmarks/` — benchmark manifests and baselines
- `scripts/` — gate, benchmark, and release scripts
- `docs/` — focused technical docs
- `PARITY.md` — syntax parity checklist and priorities

## Quality gates and benchmarks

High-signal local workflows:

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

## Diagnostics and profiling

Useful environment switches:

- `TONIC_DEBUG_CACHE=1` — cache hit/miss traces
- `TONIC_DEBUG_MODULE_LOADS=1` — module-load traces
- `TONIC_DEBUG_TYPES=1` — type-signature summaries
- `TONIC_PROFILE_STDERR=1` — per-phase timings on stderr
- `TONIC_PROFILE_OUT=<path>` — JSONL timing output
- `TONIC_MEMORY_MODE=<append_only|rc|trace>` + `TONIC_MEMORY_STATS=1` — memory diagnostics

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

## Roadmap (short-term)

See [PARITY.md](PARITY.md) for the full tracked list. Near-term gaps include:

- Numeric literal parity (hex/octal/binary, numeric separators, char literals)
- Operator parity (`===`, `!==`, `div`, `rem`, `not in`, stepped ranges, bitwise family)
- Bitstring/binary pattern parity
- Additional compile-time/module form parity
- `tonic docs` command surface

## License

No `LICENSE` file is currently committed in this repository.

If you plan to publish Tonic as open source for external use/reuse, adding an explicit license should be treated as a release blocker.
