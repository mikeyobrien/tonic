# Task: Harden Cache and Artifact Write Safety

## Description
Improve resilience of cache/artifact persistence to prevent corruption, partial-write poisoning, and path conflict failures. Ensure safe rebuild behavior and deterministic cache key behavior across runs and targets.

## Background
Tonic relies on on-disk artifacts for warm-start performance and compile reuse. Reliability degrades quickly if artifact writes are non-atomic or if corrupted state is reused. This task makes cache behavior robust under failure conditions.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-02-20-elixir-tui-cli-language/design/detailed-design.md`

**Additional References (if relevant to this task):**
- Cache subsystem: `src/cache.rs`
- Run/compile pipeline: `src/main.rs`
- Existing cache tests: `tests/run_cache_hit_smoke.rs`, `tests/run_cache_corruption_recovery_smoke.rs`
- Compile task context: `.agents/tasks/2026-02-21-tonic-compile/add-tonic-compile.code-task.md`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Ensure write path is atomic where practical (temp file + rename) to avoid partial artifact exposure.
2. Ensure directory/file conflicts are handled safely with deterministic diagnostics.
3. Ensure corruption recovery always evicts invalid artifacts and recompiles cleanly.
4. Preserve deterministic cache key dimensions (entry/deps/runtime version/target/flags).
5. Validate target segregation (`os-arch`) remains enforced.
6. Add tests for permission-denied and unwritable artifact directories.
7. Ensure failures do not leave inconsistent state that blocks subsequent successful runs.

## Dependencies
- `src/cache.rs`
- `src/main.rs` run/compile integration
- Existing cache integration tests in `tests/`

## Implementation Approach
1. Refactor write path to temporary file then atomic rename.
2. Add explicit handling for path conflicts and permission failures.
3. Keep corruption fallback behavior strict and tested.
4. Add negative-path integration tests using temp dirs with controlled failures.

## Acceptance Criteria

1. **Corrupted Artifact Recovery**
   - Given a corrupted artifact in cache
   - When Tonic run/check/compile uses that entry
   - Then the artifact is ignored/evicted and the pipeline succeeds via recompile

2. **Atomic Write Behavior**
   - Given artifact generation under normal conditions
   - When write completes
   - Then only complete artifacts are visible at target paths

3. **Path Conflict Diagnostics**
   - Given a directory/file collision at artifact path
   - When persistence runs
   - Then Tonic reports deterministic actionable diagnostics

4. **Target-Aware Cache Isolation**
   - Given equivalent sources across different targets
   - When artifacts are keyed/stored
   - Then target-specific keys prevent cross-target artifact reuse

5. **Cache Reliability Test Coverage**
   - Given the hardened cache subsystem
   - When `cargo test` runs
   - Then cache hit/miss/corruption/conflict tests pass

## Metadata
- **Complexity**: Medium
- **Labels**: Cache, Artifact, Atomic Writes, Target Isolation, Reliability
- **Required Skills**: Rust IO/fs semantics, atomic file strategies, robust error handling, integration testing