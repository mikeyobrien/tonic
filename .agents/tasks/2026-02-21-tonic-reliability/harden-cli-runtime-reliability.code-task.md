# Task: Harden Tonic CLI Runtime Reliability for Script/Project Workloads

## Description
Implement a reliability-hardening pass focused on Tonic’s actual target: fast, deterministic CLI/script execution for single files and project roots. The goal is to reduce operational failure modes (bad manifests, cache corruption, unstable diagnostics, runaway shell-outs, flaky contracts) without introducing actor/OTP runtime complexity that is out of scope for v0.

## Background
A re-review of project scope and planning artifacts shows Tonic explicitly defers OTP/process runtime features in v0. Current priorities are language-core correctness, startup/memory gates, deterministic command behavior, and robust build/run/check workflows.

This task replaces OTP-lite ambitions with practical reliability work aligned to current product goals:
- fast startup and low memory
- deterministic diagnostics and exit codes
- resilient cache and artifact handling
- stable project/module loading behavior

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Scope decisions: `.agents/planning/2026-02-20-elixir-tui-cli-language/idea-honing.md`
- Implementation plan: `.agents/planning/2026-02-20-elixir-tui-cli-language/implementation/plan.md`
- Product research constraints: `research.md`
- Runtime architecture context: `research/runtime-architecture.md`
- Current command/runtime pipeline: `src/main.rs`, `src/manifest.rs`, `src/cache.rs`, `src/runtime.rs`, `src/cli_diag.rs`
- Existing integration contracts: `tests/`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Enforce deterministic error contracts (message shape + exit codes) across `run`, `check`, `test`, `fmt`, and `compile` (if present).
2. Improve manifest/project loading resilience:
   - missing `tonic.toml`
   - invalid TOML
   - missing/empty `project.entry`
   - unreadable source files
   - recursive/ambiguous module loading edge cases
3. Harden cache subsystem behavior:
   - corruption-safe fallback is guaranteed
   - partial writes do not poison future runs
   - cache key dimensions remain deterministic (source/deps/version/target/flags)
4. Add explicit run-time budget controls for external command execution paths (if shell/process interop exists), including timeout and clear timeout diagnostics.
5. Ensure `tonic check` and `tonic run` behavior for file vs project root paths remains deterministic and well-documented.
6. Validate and stabilize all dump modes (`--dump-tokens`, `--dump-ast`, `--dump-ir`) including mutual exclusion and serialization error paths.
7. Add benchmark-gate enforcement wiring checks so regressions in startup/memory thresholds fail verify gates deterministically.
8. Add robust filesystem output handling for compile/cache artifacts:
   - parent dir creation
   - directory-vs-file conflicts
   - permission failures
   - path reporting consistency
9. Keep implementation focused on CLI/script reliability; do not introduce actor runtime abstractions.
10. Preserve current language semantics and existing interpreter behavior (`ok/err`, `?`, case, collections/protocol dispatch).

## Dependencies
- Existing CLI and diagnostics surface (`src/main.rs`, `src/cli_diag.rs`)
- Existing source/project loader (`src/manifest.rs`)
- Existing cache implementation (`src/cache.rs`)
- Existing runtime evaluator (`src/runtime.rs`)
- Existing test harness/integration style (`tests/`)
- Serialization libraries (`serde`, `serde_json`, `serde_yaml`, `toml`)

## Implementation Approach
1. **Codify reliability contracts first**
   - Document expected stdout/stderr/exit-code contracts per command.
   - Add or tighten integration tests before behavior changes.

2. **Manifest + loader hardening**
   - Normalize loader errors and ensure deterministic text for common failures.
   - Add tests for malformed project structures and edge-case paths.

3. **Cache hardening**
   - Ensure corrupted artifacts are evicted and safely rebuilt.
   - Add tests for truncation/invalid JSON/permission issues.

4. **Command behavior consistency**
   - Ensure command argument validation and help flows are consistent.
   - Keep dump mode behavior strict and mutually exclusive.

5. **Timeout/interop safeguards (if applicable)**
   - Add explicit timeout policy for subprocess/shell interop points.
   - Emit clear error diagnostics on timeout or non-zero execution status.

6. **Performance gate reliability**
   - Confirm verify-path threshold enforcement for cold/warm startup and RSS.
   - Add tests for pass/fail threshold handling and structured report output.

7. **Regression sweep**
   - Run full test suite and ensure no semantics regressions.
   - Add targeted fixtures for previously brittle paths.

## Acceptance Criteria

1. **Deterministic Command Failures**
   - Given invalid input for any supported command
   - When the command fails
   - Then stderr and exit code match a deterministic, tested contract

2. **Manifest Validation Reliability**
   - Given malformed or incomplete `tonic.toml`
   - When `tonic run <project-root>` or `tonic check <project-root>` executes
   - Then command fails with clear validation diagnostics and stable exit status

3. **Project Loader Robustness**
   - Given multi-module and edge-case file trees
   - When project source loading runs
   - Then module discovery is deterministic and does not silently skip required sources

4. **Cache Corruption Recovery**
   - Given a corrupted cache artifact for a valid source
   - When `tonic run` executes
   - Then the runtime ignores/repairs bad cache state and succeeds via recompile path

5. **Dump Mode Contract Stability**
   - Given each dump mode and invalid combinations
   - When `tonic check` executes
   - Then output format and validation errors follow tested deterministic behavior

6. **Artifact Path Handling**
   - Given cache/compile artifact path conflicts or permission failures
   - When writing artifacts
   - Then command returns actionable diagnostics without leaving inconsistent state

7. **Performance Gate Enforcement**
   - Given benchmark metrics above configured thresholds
   - When verify gate runs
   - Then verification fails deterministically with explicit threshold failure reporting

8. **No Scope Creep to OTP Runtime**
   - Given this reliability-hardening milestone
   - When implementation is complete
   - Then no actor/process/supervisor runtime subsystem is introduced

9. **Regression Safety**
   - Given existing language/runtime and CLI fixtures
   - When `cargo test` is run
   - Then existing behavior remains green and new reliability tests also pass

## Metadata
- **Complexity**: Medium
- **Labels**: Reliability, CLI, Runtime, Cache, Manifest, Diagnostics, Verification
- **Required Skills**: Rust error handling, filesystem robustness, command-contract testing, integration testing, performance-gate validation