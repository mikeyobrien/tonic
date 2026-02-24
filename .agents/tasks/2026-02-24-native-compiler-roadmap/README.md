# Native Compiler Roadmap — Rust/Go-Class Performance

Goal: move Tonic from IR-interpreted execution to an LLVM-backed AOT compiler path that can compete with Rust/Go for representative CLI workloads while preserving deterministic diagnostics and language semantics.

## Sequence

1. `01-performance-targets-and-gap-baseline.code-task.md`
2. `02-mir-cfg-and-typed-lowering.code-task.md`
3. `03-runtime-value-abi-and-memory-model.code-task.md`
4. `04-native-runtime-library-core-primitives.code-task.md`
5. `05-llvm-backend-mvp-int-bool.code-task.md`
6. `06-llvm-backend-control-flow-and-calls.code-task.md`
7. `07-llvm-backend-data-structures-and-pattern-match.code-task.md`
8. `08-llvm-backend-errors-question-try-raise.code-task.md`
9. `09-llvm-backend-closures-and-captures.code-task.md`
10. `10-host-interop-ffi-abi.code-task.md`
11. `11-aot-artifacts-and-cli-integration.code-task.md`
12. `12-differential-correctness-and-fuzzing.code-task.md`
13. `13-optimization-and-startup-tuning.code-task.md`
14. `14-rust-go-competitive-gates-and-ci.code-task.md`

## Milestones

- **M1 (Tasks 1–4):** compiler architecture foundation + runtime ABI/library
- **M2 (Tasks 5–9):** feature-complete LLVM backend for language core
- **M3 (Tasks 10–12):** production safety, interop, differential correctness
- **M4 (Tasks 13–14):** optimization and hard performance gates versus Rust/Go baselines

## Done Definition (per task)

- `cargo fmt --all` clean
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo test` green
- Bench/differential checks for the scope are green
- One conventional commit per task
