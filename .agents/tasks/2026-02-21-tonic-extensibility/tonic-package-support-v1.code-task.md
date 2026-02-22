# Task: Add Tonic-Native Package Support (v1) with `path` + `git` Dependencies

> **Status:** Done
> `HEARTBEAT_TASK_STATUS=done`


## Description
Implement a Tonic-native dependency system in `tonic.toml` that supports local path dependencies and pinned git dependencies, with lockfile-based reproducibility and deterministic module loading.

## Background
Tonic currently supports single-file and project-root execution but lacks dependency management. For real usage, teams need a reproducible package workflow. Full Mix/Hex compatibility is out of scope; the v1 solution should be native to Tonic and optimized for deterministic CLI workflows.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Scope constraints and non-goals: `.agents/planning/2026-02-20-elixir-tui-cli-language/idea-honing.md`
- Project loader and manifest parsing: `src/manifest.rs`
- CLI command routing: `src/main.rs`
- Cache keying and artifact persistence: `src/cache.rs`
- Compile task context: `.agents/tasks/2026-02-21-tonic-compile/add-tonic-compile.code-task.md`
- Reliability tasks: `.agents/tasks/2026-02-21-tonic-reliability/`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Extend `tonic.toml` schema to support dependencies:
   - `[dependencies]`
   - path form: `foo = { path = "../foo" }`
   - git form: `bar = { git = "https://...", rev = "<sha>" }` (rev required in v1 for determinism)
2. Introduce `tonic.lock` lockfile capturing resolved dependency sources and immutable revisions.
3. Implement deterministic dependency resolution order and conflict handling.
4. Add dependency fetch/store workspace under `.tonic/deps/` for git dependencies.
5. Integrate dependencies into source aggregation/module loading so `run/check/compile` see dependency modules.
6. Add duplicate module-name and shadowing diagnostics across root project and dependencies.
7. Add cache-key influence from dependency graph/lockfile content so stale artifacts are invalidated correctly.
8. Provide minimal dependency CLI commands (at least one of these models):
   - `tonic deps sync`
   - `tonic deps fetch`
   - `tonic deps lock`
   (choose one clear v1 UX and document it)
9. Ensure offline/reproducible behavior with lockfile present and fetched deps available.
10. Preserve startup and deterministic behavior goals; avoid introducing dynamic package execution semantics.

## Dependencies
- Manifest model/parser (`src/manifest.rs`)
- CLI routing (`src/main.rs`)
- Cache keying (`src/cache.rs`)
- Filesystem/network fetch layer (new module likely required)
- Existing integration test harness (`tests/`)

## Implementation Approach
1. Add manifest schema types for dependency declarations.
2. Implement resolver module for dependency graph and deterministic ordering.
3. Implement lockfile read/write with stable serialization.
4. Implement path dependency ingestion and validation first.
5. Implement git dependency fetch with pinned rev and local cache under `.tonic/deps/`.
6. Merge dependency source trees into module loading pipeline with duplicate module checks.
7. Extend cache key generation to include lockfile/dependency fingerprint.
8. Add dependency command(s) and help text.
9. Add integration tests for path+git workflows, lockfile determinism, and failure modes.

## Acceptance Criteria

1. **Path Dependency Resolution**
   - Given a project with valid path dependencies in `tonic.toml`
   - When `tonic run` or `tonic check` executes
   - Then dependency modules are loaded deterministically and project execution succeeds

2. **Pinned Git Dependency Resolution**
   - Given a project with git dependencies pinned by `rev`
   - When dependency sync/fetch runs
   - Then sources are fetched into `.tonic/deps/` and used by compile/run deterministically

3. **Lockfile Reproducibility**
   - Given the same manifest and dependency state
   - When lockfile is generated twice
   - Then `tonic.lock` content is stable and equivalent

4. **Duplicate Module Detection**
   - Given conflicting module names across root and dependencies
   - When loading/compiling
   - Then Tonic fails with explicit conflict diagnostics

5. **Cache Invalidation on Dependency Change**
   - Given a dependency revision or lockfile change
   - When run/compile executes
   - Then cached artifacts are invalidated and rebuilt correctly

6. **Offline Behavior with Warm Dependency Cache**
   - Given dependencies are already fetched and lockfile exists
   - When network is unavailable
   - Then project still runs/checks successfully using local dependency cache

7. **Failure Diagnostics for Dependency Errors**
   - Given bad dependency declarations (missing path, missing rev, unreachable git source)
   - When dependency resolution runs
   - Then deterministic actionable diagnostics are emitted

8. **Regression Safety**
   - Given existing no-dependency projects and command fixtures
   - When full tests run
   - Then prior behaviors remain unchanged

## Metadata
- **Complexity**: High
- **Labels**: Package Management, Dependencies, Lockfile, Determinism, Build System
- **Required Skills**: Rust manifest parsing, dependency graph design, reproducible builds, filesystem/network robustness, integration testing