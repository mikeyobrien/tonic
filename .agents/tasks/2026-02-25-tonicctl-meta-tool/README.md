# Tonicctl Meta-Tool (Executable) — Task Sequence

Goal: evolve `examples/apps/tonicctl` from a pure plan emitter into an executable meta-tool for Tonic repositories.

## Why this backlog exists

The current example can describe workflows, but cannot execute them. To become operational, tonicctl needs:
- system capabilities (process/filesystem/env) exposed safely to Tonic code
- a small stdlib-facing system API layer
- command dispatch + deterministic exit semantics
- strict benchmark and release dry-run orchestration

## Sequence

1. `01-system-capability-contract-and-safety-model.code-task.md`
2. `02-host-interop-process-exec-and-exit-contract.code-task.md`
3. `03-host-interop-filesystem-primitives-and-artifact-io.code-task.md`
4. `04-host-interop-env-and-tool-discovery-primitives.code-task.md`
5. `05-stdlib-system-module-v1-for-tonicctl.code-task.md`
6. `06-tonicctl-cli-arg-dispatch-and-command-contract.code-task.md`
7. `07-tonicctl-doctor-command-runtime-execution.code-task.md`
8. `08-tonicctl-gates-and-bench-strict-command-execution.code-task.md`
9. `09-tonicctl-release-dry-run-and-reporting.code-task.md`
10. `10-tonicctl-e2e-tests-ci-docs-and-example-contract.code-task.md`

## Definition of Done

- `tonicctl doctor|gates|bench --strict|release --dry-run` execute with deterministic output and non-zero failures when contracts fail.
- `tonic compile examples/apps/tonicctl` (or `--out`) produces a runnable native executable preserving behavior.
- Interpreter and compiled runs of tonicctl agree for core success/failure paths.
- CI/docs are updated and no stale legacy compile flag wording is introduced.
