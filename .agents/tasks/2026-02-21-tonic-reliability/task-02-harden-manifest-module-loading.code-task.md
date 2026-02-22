HEARTBEAT_TASK_STATUS=done
# Task: Harden Manifest Validation and Module Loading

## Status
**Completed**
- Hardened `tonic.toml` parsing with distinct errors for missing file, invalid syntax, missing entry, and empty entry strings.
- Added validation to ensure the entry path exists and is a file.
- Ensured `collect_tonic_source_paths` ignores hidden (`.*`) and `target` directories, restricting traversal to actual project source directories.
- Relied on the existing name resolver validation to catch and explicitly reject duplicate module definitions with `E1003` deterministically.
- Cleaned up unused functions to adhere strictly to Rust idioms and clippy warnings.
- Wrote regression tests comprehensively covering the new manifest and module loader constraints.

## Description
Strengthen project-root loading behavior so `tonic run <project-root>` and `tonic check <project-root>` fail clearly on invalid structure and behave deterministically for valid multi-module trees.

## Background
Project loading is a high-frequency path for Tonic users. Failure modes around `tonic.toml`, entry resolution, file readability, and module discovery can silently degrade reliability. This task makes loader behavior explicit and robust.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Scope decisions: `.agents/planning/2026-02-20-elixir-tui-cli-language/idea-honing.md`
- Project source loading code: `src/manifest.rs`
- Parser/AST helpers used by loader analysis: `src/parser.rs`, `src/lexer.rs`
- Existing project-path tests: `tests/run_project_multimodule_smoke.rs`, `tests/run_manifest_validation.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Harden manifest parsing errors for:
   - missing `tonic.toml`
   - invalid TOML
   - missing `project.entry`
   - empty/whitespace `project.entry`
2. Validate entry path semantics (exists, file, readable) with deterministic diagnostics.
3. Ensure module discovery order is deterministic across filesystems.
4. Prevent ambiguous module loading behavior (e.g., duplicate module definitions) with explicit errors.
5. Ensure directory traversal only includes intended `.tn` sources.
6. Preserve lazy stdlib loading behavior and tracing contracts.
7. Add integration tests for edge cases and negative paths.

## Dependencies
- `src/manifest.rs`
- `src/main.rs` command handlers using loader
- Existing loader-related tests and fixtures in `tests/`

## Implementation Approach
1. Enumerate and normalize loader error messages by failure class.
2. Add validation checks around entry path and module aggregation.
3. Keep sorting/determinism guarantees explicit in code and tests.
4. Add fixture-driven tests for malformed project trees and duplicate module names.

## Acceptance Criteria

1. **Manifest Validation Errors Are Deterministic**
   - Given invalid or incomplete `tonic.toml`
   - When project commands execute
   - Then Tonic emits deterministic validation errors and failure exit status

2. **Entry Resolution Is Explicit and Safe**
   - Given a manifest entry pointing to a missing/unreadable/non-file path
   - When project commands execute
   - Then Tonic fails with actionable diagnostics

3. **Module Loading Is Deterministic**
   - Given the same valid project tree on repeated runs
   - When module loading occurs
   - Then discovered module order and behavior are deterministic

4. **Ambiguous Module Definitions Are Rejected**
   - Given duplicate module definitions in project sources
   - When compilation/checking runs
   - Then Tonic fails with explicit ambiguity diagnostics

5. **Loader Regression Coverage Exists**
   - Given the hardened loader implementation
   - When `cargo test` runs
   - Then loader reliability tests pass consistently

## Metadata
- **Complexity**: Medium
- **Labels**: Manifest, Module Loader, Determinism, Reliability, Testing
- **Required Skills**: Rust filesystem handling, parser pipeline integration, fixture-based integration testing