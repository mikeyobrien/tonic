# Tonic Reliability Regression Matrix & Contributor Guidance

## Overview

This document codifies the core reliability failure classes for the Tonic v0 runtime, mapping them directly to our integration suite. It also serves as guidance for contributors to ensure that future features and commands maintain our deterministic command contracts.

Tonic v0 is built with explicit non-goals: it does not include an OTP runtime, a macro system, or dynamic evaluation. Reliability in v0 is strictly focused on language-core stability, startup performance, and predictable CLI behavior.

## Regression Matrix

The following table maps our critical failure classes to the concrete tests that verify their behavior. When modifying the core runtime or adding features, ensure these tests continue to pass and are expanded if new failure modes are introduced.

| Failure Class | Description | Concrete Tests (`tests/`) |
| :--- | :--- | :--- |
| **Command Contracts** | CLI usage errors, invalid arguments, missing files, expected exit codes (0 for success, 1 for runtime error, 64 for usage). | `cli_contract_common.rs`, `cli_contract_compile.rs`, `cli_contract_run_command.rs` |
| **Manifest / Loader** | Missing manifests, unresolvable module dependencies, duplicate module conflicts, and deterministic lockfile generation. | `run_manifest_validation.rs`, `run_dependency_duplicate_module_conflict.rs`, `deps_lockfile_determinism.rs`, `deps_manifest_dependency_diagnostics.rs` |
| **Cache / Artifacts** | Missing cache directories, corrupt cache artifacts, permission denied errors, offline cache hits, and cache path conflicts. | `run_cache_corruption_recovery_smoke.rs`, `run_cache_permission_denied_smoke.rs`, `run_cache_path_conflict_smoke.rs`, `run_dependency_offline_warm_cache.rs` |
| **Backend Differential Correctness** | Semantic drift between interpreter and native LLVM/AOT execution paths, including generated-program regressions. | `differential_backends.rs` |
| **Verify / Dump** | Verification commands failing to find files, invalid AST/IR dumps, and mode-specific verification issues. | `check_dump_ast_*.rs`, `check_dump_ir_*.rs`, `verify_auto_mode_json.rs`, `verify_manual_evidence_*.rs` |

## Contributor Guidance: Preserving Command Contracts

To maintain a reliable CLI experience, contributors must adhere to the following contracts:

### 1. Deterministic Exit Codes
Every command execution must map its outcome to a predictable exit code:
- **0**: Success.
- **1**: Runtime or compilation error (user error within the code being executed).
- **64**: CLI Usage error (e.g., missing arguments, unknown flags).

### 2. Output Guarantees
- Standard output (`stdout`) is reserved for the primary result of the command (e.g., the output of the executed script or JSON verification output).
- Standard error (`stderr`) is reserved for diagnostics, warnings, panic messages, and compilation errors.
- Do not introduce flaky logging that violates cache-hit expectations unless explicitly debugging.

### 3. Reliability Gates (Testing)
When adding a new command or a significant feature:
1. **Reuse Fixture Helpers:** Use the consolidated `tests/common/mod.rs` utilities (like `common::unique_fixture_root`) rather than creating ad-hoc test directories.
2. **Add CLI Contract Tests:** If you add a new command, add its signature to `cli_contract_common.rs` to ensure it gracefully handles missing/extra arguments and returns exit code 64.
3. **Verify Edge Cases:** Specifically test permission denials, missing files, and unparseable configurations to ensure the system does not panic but returns a controlled error state.
4. **Run Differential Gate for Backend Changes:** Execute `scripts/differential-enforce.sh` (or `cargo test --test differential_backends -- --nocapture`) when touching codegen/runtime paths to block semantic regressions.
5. **Run Native Competitive Gate Before Release Work:** Execute `scripts/native-gates.sh` (or benchmark + `scripts/native-regression-policy.sh --mode strict`) when touching compile/runtime hot paths to preserve Rust/Go comparative contracts.

### 4. Non-Goals
Remember that Tonic v0 is designed for CLI scripting and small applications:
- **Do not introduce background processes or OTP supervision trees.**
- **Do not add dynamic code loading.** If code cannot be statically resolved and typed, it should not be merged into v0.
- If an issue is fundamentally related to these non-goals, document the limitation rather than attempting a brittle workaround.

By adhering to this matrix and these guidelines, Tonic ensures a predictable, fast, and stable foundation.
