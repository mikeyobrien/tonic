# Architecture

## 1) High-level architecture

`tonic` is a **single Rust crate** implementing:

1. A Tonic language frontend (lex/parse/resolve/type/lower).
2. Two execution paths:
   - **Interpreter path** (`tonic run` evaluates IR directly).
   - **Native path** (`tonic compile` lowers IR→MIR→C/LLVM artifacts, links executable).
3. Quality gates and parity tooling (benchmarks, differential checks, LLVM catalog parity).

```mermaid
graph TD
    CLI[tonic CLI]

    CLI --> LOAD[manifest/source loading]
    LOAD --> LEX[lexer]
    LEX --> PARSE[parser AST]
    PARSE --> RESOLVE[resolver]
    RESOLVE --> TYPE[type inference]
    TYPE --> IR[IR lowering]

    IR --> RUN[interpreter runtime]
    IR --> MIR[MIR lowering]
    MIR --> OPT[MIR optimization]

    OPT --> LLVM[LLVM IR generation]
    OPT --> CBACK[C backend generation]
    CBACK --> LINK[system C compiler/linker]
    LINK --> EXE[native executable]

    RUN --> OUT1[stdout / diagnostics]
    EXE --> OUT2[stdout / diagnostics]
```

## 2) Command architecture

- `run`: source/manifest load → frontend pipeline → IR eval (`runtime.rs`).
- `check`: same frontend pipeline, optional dump modes (`tokens/ast/ir/mir`).
- `test`: test file discovery + compile suite + execute discovered `test_*` functions.
- `fmt`: deterministic source reformatter over `.tn` files.
- `compile`: frontend + MIR + backend sidecars + native executable.
- `verify`: acceptance metadata + benchmark/manual evidence policy gate.
- `deps`: lockfile + dependency sync (path/git).

## 3) Frontend architecture (language)

```mermaid
sequenceDiagram
    participant CLI as tonic command
    participant M as manifest loader
    participant L as lexer
    participant P as parser
    participant R as resolver
    participant T as typing
    participant I as IR lowerer

    CLI->>M: load_run_source(path or project)
    M-->>CLI: concatenated source
    CLI->>L: scan_tokens(source)
    L-->>CLI: Token[]
    CLI->>P: parse_ast(tokens)
    P-->>CLI: AST
    CLI->>R: resolve_ast(AST)
    R-->>CLI: semantic validation
    CLI->>T: infer_types(AST)
    T-->>CLI: TypeSummary
    CLI->>I: lower_ast_to_ir(AST)
    I-->>CLI: IrProgram
```

### Notable frontend decisions

- Parser supports Elixir-like constructs (module forms, comprehensions, try/rescue/catch/after, protocol/defimpl syntax).
- Resolver emits stable diagnostic families (`E1xxx`) and checks imports/protocols/visibility.
- Type checker emits stable `E2xxx/E3xxx` diagnostics and validates `?` operator + case exhaustiveness.

## 4) Runtime architecture

There are two runtime layers:

1. **Interpreter runtime (`runtime.rs`)**
   - Stack-based IR op evaluation.
   - Runtime value model (`RuntimeValue`) includes ints, strings, lists/maps, tuples, results, closures.

2. **Native runtime helpers (`native_runtime/*`, `native_abi/*`)**
   - Shared operation helpers (`ops`, `collections`, `pattern`, `interop`).
   - C ABI-safe boundary (`TCallContext`/`TCallResult`, `TValue`, ABI version checks).

```mermaid
graph LR
    IR[IrProgram] --> INT[runtime.rs evaluator]
    INT --> NOPS[native_runtime::ops]
    INT --> NCOL[native_runtime::collections]
    INT --> NPAT[native_runtime::pattern]
    INT --> NINT[native_runtime::interop]

    NINT --> HOST[HostRegistry + system host fns]
    NOPS --> ABI[native_abi boundary helpers]
```

## 5) Backend architecture

### C backend

- `c_backend::lower_mir_to_c` builds a self-contained C source file.
- Includes runtime stubs/helpers and function dispatchers for multi-clause functions.
- `linker.rs` finds `clang/gcc/cc` and produces executable.

### LLVM backend

- `llvm_backend::lower_mir_subset_to_llvm_ir` emits LLVM IR text (subset-focused).
- Handles dispatcher generation, helper declarations, and main entrypoint.
- Parity validated through `llvm_catalog_parity` tooling.

## 6) Artifact architecture

`tonic compile` outputs to `.tonic/build/<stem>` by default:

- `<stem>.ll` (LLVM sidecar)
- `<stem>.c` (C source sidecar)
- `<stem>.tir.json` (IR sidecar)
- `<stem>.tnx.json` (native artifact manifest)
- `<stem>` (native executable)

## 7) Operational architecture

Quality/release is script-driven:

- `scripts/native-gates.sh` orchestrates fmt/clippy/tests/differential/parity/benchmark policy/memory guardrails.
- `scripts/release-alpha-readiness.sh` validates clean repo, changelog, and gate artifacts before alpha tag.

For operational flows, see `./workflows.md`.
