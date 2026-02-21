# Task: Add `tonic compile` Command for Files and Project Roots

## Description
Implement a first-class `tonic compile` command that compiles a `.tn` file or a project root (directory with `tonic.toml`) into a reusable on-disk artifact. This closes a current CLI gap where compilation only happens implicitly inside `tonic run` and cannot be invoked directly by users or CI workflows.

## Background
The existing CLI supports `run`, `check`, `test`, `fmt`, `cache`, and `verify`. Internally, the frontend pipeline already exists (`scan_tokens` -> `parse_ast` -> `resolve_ast` -> `infer_types` -> `lower_ast_to_ir`) and produces `IrProgram`. However, there is no explicit compile UX to produce and persist a build artifact for either:
- single-file script workflows, or
- multi-module project workflows loaded via `tonic.toml`.

A dedicated compile command is needed to make build behavior explicit, improve automation ergonomics, and provide a stable output contract for downstream tooling.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `.agents/planning/2026-02-20-elixir-tui-cli-language/implementation/plan.md` (Step 8, 11, 12, and CLI workflow expectations)
- `src/main.rs` (current CLI routing and compile pipeline)
- `src/manifest.rs` (file/project source loading behavior)
- `src/cache.rs` (existing serialization and artifact persistence patterns)
- `tests/` integration tests for command contracts and output behavior

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Add a new top-level CLI command: `tonic compile <path> [--out <artifact-path>]`.
2. Update top-level help output and command routing to include `compile`.
3. Add dedicated help output for compile usage and argument contract.
4. Support both compile targets:
   - a direct `.tn` source file path
   - a project root directory path resolved through `tonic.toml` + module loading
5. Reuse the existing frontend compile pipeline (lex/parse/resolve/type/lower) without changing language semantics.
6. Emit a deterministic serialized artifact to disk (minimum: lowered `IrProgram`; preferred: versioned envelope containing IR + metadata).
7. Provide a deterministic default output location when `--out` is not provided (e.g., `.tonic/build/<entry-stem>.tir.json`), creating parent directories as needed.
8. Support explicit output override via `--out` and validate path usability.
9. Emit a stable success contract on stdout (e.g., `compile: ok <artifact-path>`) and use existing diagnostic conventions on failure.
10. Preserve existing behavior of `run`, `check`, `test`, `fmt`, `cache`, and `verify` (no regressions).
11. Keep new implementation files under 500 LOC where practical; if main CLI routing grows further, factor compile behavior into a focused module.
12. Add tests that cover compile functionality end-to-end, including project-root compilation and artifact content validation.

## Dependencies
- Existing compiler pipeline functions and data structures (`scan_tokens`, `parse_ast`, `resolve_ast`, `infer_types`, `lower_ast_to_ir`, `IrProgram`)
- Existing source-loading behavior (`load_run_source`) for file/project support
- `serde`/`serde_json` for artifact serialization
- Filesystem/path APIs (`std::fs`, `std::path`) for output path resolution and directory creation
- Existing CLI diagnostic and exit-code contracts (`cli_diag`)

## Implementation Approach
1. **Define compile command contract**
   - Add `compile` to command dispatch in `run(args)`.
   - Implement `handle_compile(args)` with path parsing, optional `--out`, usage/validation behavior, and help output.

2. **Extract/reuse compile pipeline cleanly**
   - Reuse existing `compile_source_to_ir` logic; if needed, move it into a shared module so both `run` and `compile` use the same code path.
   - Ensure compile command performs identical semantic checks as `run` pre-execution.

3. **Implement artifact schema and serialization**
   - Define a stable artifact format (minimum: IR JSON; preferred: envelope with schema/compiler/source metadata).
   - Serialize deterministically and fail with actionable diagnostics when writing fails.

4. **Implement output path strategy**
   - Resolve default output path from compile target.
   - Ensure directories are created and output files are overwritten safely.
   - Respect and validate `--out` override.

5. **Wire user-facing output and exit behavior**
   - Success: print stable `compile: ok ...` contract.
   - Failure: route through `CliDiagnostic::failure(...)` with deterministic exit code.

6. **Add automated tests (unit + integration)**
   - CLI routing/help tests for new command visibility and usage.
   - Integration test: `tonic compile examples/simple.tn` writes expected artifact.
   - Integration test: `tonic compile .` in a fixture project writes artifact for project entry.
   - Integration test: `--out` writes to custom path.
   - Integration test: invalid path / missing manifest produces expected error message.
   - Artifact snapshot/content test validates serialized IR structure and key metadata fields.

7. **Regression verification**
   - Run full test suite to confirm no command contract regressions.
   - Ensure existing `run` behavior remains unchanged.

## Acceptance Criteria

1. **Compile Command Discoverability**
   - Given a user runs `tonic --help`
   - When the command list is displayed
   - Then `compile` appears with an accurate one-line description and usage support

2. **Single File Compilation**
   - Given a valid `.tn` source file
   - When `tonic compile <file>` is executed
   - Then the command exits successfully and writes a serialized compile artifact to the default location

3. **Project Root Compilation**
   - Given a valid project directory containing `tonic.toml` and an entry module
   - When `tonic compile <project-root>` is executed
   - Then the command exits successfully and writes a serialized compile artifact for the resolved project entry

4. **Custom Output Path Support**
   - Given a valid source target and a writable custom artifact path
   - When `tonic compile <path> --out <custom-path>` is executed
   - Then the artifact is written to `<custom-path>` and stdout reports that path in the success contract

5. **Deterministic Artifact Content**
   - Given the same unchanged source input compiled twice
   - When both artifacts are read
   - Then the serialized artifact structure and IR content are deterministic and equivalent

6. **Failure Diagnostics**
   - Given an invalid compile target (missing file, malformed project manifest, or unwritable output path)
   - When `tonic compile` is executed
   - Then the command exits with failure and prints a deterministic error diagnostic matching existing CLI conventions

7. **No Regressions in Existing Commands**
   - Given the existing CLI command suite
   - When integration tests for `run`, `check`, `test`, `fmt`, `cache`, and `verify` are executed
   - Then all existing command contracts continue to pass unchanged

8. **Unit and Integration Test Coverage**
   - Given the compile command implementation
   - When `cargo test` is run
   - Then compile command behavior (help/routing, file/project compilation, `--out`, failure paths, artifact content) is covered by automated tests and all tests pass

## Metadata
- **Complexity**: High
- **Labels**: CLI, Compiler, Build Artifact, Project Loader, DX, Testing
- **Required Skills**: Rust CLI design, serialization contracts, filesystem/path handling, integration testing, compiler pipeline integration