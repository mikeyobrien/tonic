# AGENTS Guide for `tonic`

This file is optimized for AI coding assistants operating in this repository.

## Table of contents

| Section | Summary |
|---|---|
| [1. Preserved project directives](#1-preserved-project-directives) | Manual rules that already existed in this file and remain authoritative. |
| [2. Repo orientation](#2-repo-orientation) | Where core code, tests, docs, and scripts live. |
| [3. Command quickstart](#3-command-quickstart) | High-signal commands for run/check/test/compile/deps/verify. |
| [4. Architecture snapshot](#4-architecture-snapshot) | End-to-end pipeline and subsystem ownership map. |
| [5. Validation and release gates](#5-validation-and-release-gates) | Blocking checks and scripts used before release. |
| [6. Debugging and profiling hooks](#6-debugging-and-profiling-hooks) | Environment flags and diagnostics that help isolate failures. |
| [7. Task routing guide](#7-task-routing-guide) | Which file families to inspect first for common task types. |
| [8. Documentation cross-references](#8-documentation-cross-references) | Links into generated knowledge base for deeper context. |

<!-- tags: policy,manual-rules,non-negotiable -->
## 1. Preserved project directives

The following directives were already present in this file and are intentionally preserved verbatim:

- Smaller implementation files are better with a short, contextual name.
- Commit when tests pass

<!-- tags: orientation,layout,ownership -->
## 2. Repo orientation

- Core compiler/runtime: `src/`
  - Frontend: `lexer.rs`, `parser.rs`, `resolver.rs`, `typing.rs`
  - Lowering: `ir.rs`, `mir/*`
  - Backends: `llvm_backend/*`, `c_backend/*`, `linker.rs`
  - Runtime: `runtime.rs`, `native_runtime/*`, `native_abi/*`
- Integration tests: `tests/*.rs` (large contract coverage)
- Language fixtures/examples: `examples/`
- Operational scripts: `scripts/`
- Benchmark manifests and baselines: `benchmarks/`
- Project and roadmap docs: `docs/`, `PARITY.md`, `research/`

<!-- tags: commands,workflow,developer-loop -->
## 3. Command quickstart

- Run source/project:
  - `cargo run -- run <path>`
- Static checks:
  - `cargo run -- check <path>`
  - `cargo run -- check <path> --dump-ast`
- Tests:
  - `cargo test`
  - `cargo run -- test <path> [--format json]`
- Format:
  - `cargo run -- fmt <path>`
  - `cargo run -- fmt <path> --check`
- Native compile:
  - `cargo run -- compile <path> [--out <artifact>]`
- Dependencies:
  - `cargo run -- deps lock`
  - `cargo run -- deps sync`
- Acceptance verification:
  - `cargo run -- verify run <slice-id> [--mode auto|mixed|manual]`

<!-- tags: architecture,pipeline,systems -->
## 4. Architecture snapshot

Primary pipeline:

1. Source/manifest load
2. Lex (`scan_tokens`)
3. Parse (`parse_ast`)
4. Resolve (`resolve_ast`)
5. Type infer (`infer_types`)
6. Lower to IR (`lower_ast_to_ir`)
7. Either:
   - Interpret via `runtime.rs`, or
   - Lower to MIR, optimize, emit C/LLVM sidecars, link native executable.

Native artifacts default to `.tonic/build/` and include sidecars (`.ll`, `.c`, `.tir.json`, `.tnx.json`).

<!-- tags: quality,gates,release -->
## 5. Validation and release gates

Use these scripts as source-of-truth operational workflows:

- Full native gate stack: `./scripts/native-gates.sh`
  - fmt + clippy + tests + differential + llvm parity + benchmark policy + memory guardrails
- LLVM parity enforce: `./scripts/llvm-catalog-parity-enforce.sh`
- Differential backend enforce: `./scripts/differential-enforce.sh`
- Alpha readiness gate: `./scripts/release-alpha-readiness.sh --version X.Y.Z-alpha.N`

Release gate expects:

- Clean git tree
- Matching `CHANGELOG.md` heading
- Required benchmark artifacts under `.tonic/native-gates/` (or configured artifact dir)

<!-- tags: debugging,profiling,observability -->
## 6. Debugging and profiling hooks

Useful env controls:

- `TONIC_DEBUG_CACHE=1` → cache hit/miss trace
- `TONIC_DEBUG_MODULE_LOADS=1` → module load trace
- `TONIC_DEBUG_TYPES=1` → type summary count trace
- `TONIC_PROFILE_STDERR=1` → per-phase timings on stderr
- `TONIC_PROFILE_OUT=<path>` → append JSONL timing reports
- `TONIC_MEMORY_MODE=<append_only|rc|trace>` + `TONIC_MEMORY_STATS=1` → runtime memory diagnostics

<!-- tags: routing,task-selection,assistant-playbook -->
## 7. Task routing guide

- Parser or syntax behavior: start with `src/parser.rs`, then `src/lexer.rs`, tests `check_dump_ast_*`.
- Name resolution/diagnostics: `src/resolver.rs`, `src/resolver_diag.rs`, tests `check_*`.
- Type behavior: `src/typing.rs`, `src/typing_diag.rs`, tests `check_try_raise_typing.rs` and related.
- Runtime semantics: `src/runtime.rs`, `src/native_runtime/*`, tests `run_*` and `runtime_*`.
- Native compile issues: `src/mir/*`, `src/c_backend/*`, `src/llvm_backend/*`, `src/linker.rs`.
- Dependency issues: `src/manifest.rs`, `src/deps.rs`, tests `deps_*`, `run_dependency_*`.
- Bench/regression policy: `src/bin/benchsuite/*`, `scripts/native-regression-policy.sh`, benchmark manifests.

<!-- tags: references,knowledge-base,cross-links -->
## 8. Documentation cross-references

Generated codebase summary docs live at `.agents/summary/`:

- `index.md` (primary AI routing doc)
- `codebase_info.md`
- `architecture.md`
- `components.md`
- `interfaces.md`
- `data_models.md`
- `workflows.md`
- `dependencies.md`
- `review_notes.md`

Recommended AI loading order:

1. `.agents/summary/index.md`
2. One or two topic files based on user intent
3. Direct source files for implementation decisions
