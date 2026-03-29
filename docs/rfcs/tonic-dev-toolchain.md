# RFC: Developer Toolchain Improvements

## Summary

Six targeted improvements to the Tonic developer toolchain, prioritized by safety and impact: (1) install command test coverage, (2) shared test helpers, (3) error diagnostic enhancements, (4) gate script hardening, (5) stdlib inline documentation, (6) benchmark baseline management.

## Motivation

The Tonic compiler has grown a rich CLI surface (`run`, `check`, `test`, `fmt`, `install`, `deps`, `publish`, etc.) and a mature CI gate pipeline. However, several gaps threaten reliability and developer experience:

- **`tonic install`** shipped 722 lines of filesystem-mutating code with zero tests — the single largest safety gap in the repo.
- **Test authoring** requires boilerplate `Command::new(env!("CARGO_BIN_EXE_tonic"))` setup in every file; `tests/common/` has only directory helpers and differential-specific utilities.
- **Error diagnostics** have structured codes (E0002–E3002) but lack recovery hints, "did you mean?" suggestions, and machine-readable JSON output needed for editor/LLM integration.
- **Gate scripts** total ~1000+ lines of bash+embedded-Python orchestration that works but is hard to debug and extend.
- **Stdlib modules** (11 embedded in `stdlib_catalog.rs`) lack parameter/return type documentation, limiting LLM comprehension.
- **Benchmark baselines** are manually captured with no automated staleness detection.

## Design

### Work Stream 1: Install Command Test Coverage (Critical)

**Problem**: `cmd_install.rs` performs symlink creation, shim generation, manifest TOML read/write, directory creation, and PATH detection — all untested.

**Approach**: Add two test file categories following existing conventions:

1. **`tests/cli_contract_install.rs`** — CLI argument validation
   - Missing required `<source>` argument → exit 64
   - Unknown flags → exit 64
   - `--help` output includes install/uninstall/installed subcommands
   - `--copy` and `--force` flags accepted without error

2. **`tests/install_*.rs`** — Functional tests using temp directories
   - `install_local_path.rs` — Install from a fixture project dir, verify shim exists in `$TONIC_HOME/bin/`, verify `packages.toml` entry created
   - `install_uninstall.rs` — Install then uninstall, verify shim removed and manifest entry cleared
   - `install_installed_list.rs` — Install a package, run `tonic installed`, verify it appears in output
   - `install_force_overwrite.rs` — Install with existing shim from different package, verify `--force` overwrites and no-flag errors
   - `install_copy_mode.rs` — Install with `--copy`, verify source is copied (not symlinked)
   - `install_missing_source.rs` — Install from non-existent path → meaningful error, non-zero exit
   - `install_no_tonic_toml.rs` — Install from directory without `tonic.toml` → error

**Environment isolation**: All install tests MUST set `TONIC_HOME` to a temp directory (via `unique_temp_dir()`) to avoid mutating the real `~/.tonic/`. Tests clean up on drop.

**Exit code contract**: Follow existing convention — 64 for usage errors, 1 for runtime errors, 0 for success.

### Work Stream 2: Shared Test Helpers

**Problem**: 88 test files repeatedly construct `Command::new(env!("CARGO_BIN_EXE_tonic"))` with inline setup. No convenience wrappers exist.

**Approach**: Extend `tests/common/mod.rs` with:

```rust
/// Run a tonic subcommand and return the assert_cmd::Assert for chaining.
pub fn tonic_cmd(args: &[&str]) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("tonic").unwrap();
    cmd.args(args);
    cmd
}

/// Run tonic, assert exit 0, return stdout as String.
pub fn tonic_success(args: &[&str]) -> String {
    let output = tonic_cmd(args).assert().success().get_output().clone();
    String::from_utf8(output.stdout).unwrap()
}

/// Create a temp dir with TONIC_HOME set, suitable for install tests.
pub fn isolated_tonic_home() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = unique_temp_dir();
    let home = dir.path().join(".tonic");
    std::fs::create_dir_all(&home).unwrap();
    (dir, home)
}
```

These wrap the existing `assert_cmd` dependency and `unique_temp_dir()` helper. Existing tests are NOT migrated (avoid churn); new tests use the helpers.

### Work Stream 3: Error Diagnostic Enhancements

**Problem**: Error codes exist but messages lack actionable guidance and machine-readable format.

**Approach** (incremental, three sub-phases):

**3a. Recovery hints for high-frequency errors**
Add a `hint` field to the existing diagnostic types. Start with the 5 most common error patterns:
- `E1001` (UndefinedSymbol) → "Did you mean `<closest_match>`?" using Levenshtein distance on in-scope names
- `E1002` (UndefinedModule) → List available stdlib modules
- `E2001` (TypeMismatch) → Show expected vs actual with conversion suggestion if applicable
- `E1008` (PrivateFunction) → "Make it public with `pub fn`" or "Import from the correct module"
- `E0003` (UnexpectedToken) → Show what tokens were expected

**3b. Filename in all diagnostic paths**
Audit `cli_diag.rs` render path to ensure `filename:line:col` appears in every diagnostic, not just some.

**3c. JSON diagnostic output**
Add `--format json` flag to `tonic check` that emits diagnostics as JSON lines:
```json
{"code":"E1001","severity":"error","message":"undefined symbol `foo`","hint":"Did you mean `foo_bar`?","file":"main.tn","line":5,"col":3,"span_start":42,"span_end":45}
```
This reuses the existing `--format json` pattern from `--dump-tokens --format json`.

### Work Stream 4: Gate Script Hardening

**Problem**: Gate scripts work but have accumulated complexity. Full rewrite is high-risk; incremental hardening is safer.

**Approach**:

1. **Remove orphaned `bench-enforce.sh`** — Not referenced from `native-gates.sh` or CI. Delete it.
2. **Externalize remaining hardcoded thresholds** — Move budget ratios and quarantine margins from Python embedded in `native-regression-policy.sh` to `benchmarks/native-compiler-suite.toml` under a new `[regression_policy]` section.
3. **Add `set -euo pipefail`** to any script missing it (defensive).
4. **Replace `gtime` dependency** — Use `/usr/bin/time -l` on macOS natively instead of requiring GNU time, or detect and fall back gracefully.
5. **Do NOT rewrite scripts in Rust** — The bash+Python approach is load-bearing for CI. A `tonic gate` command is a future consideration, not in scope here.

### Work Stream 5: Stdlib Inline Documentation

**Problem**: 11 stdlib modules embedded in `stdlib_catalog.rs` as string constants lack parameter and return type documentation.

**Approach**: Add doc comments inside the embedded Tonic source strings following this pattern:

```tonic
## Converts a value to its string representation.
##
## Parameters:
##   value: any — the value to convert
##
## Returns: string
fn to_string(value) { ... }
```

**Prioritization** (by `docs/core-stdlib-gap-list.md`):
1. String module — most used, most gaps
2. List module — second most used
3. Map module
4. IO module
5. Remaining modules

This is documentation-only — no behavioral changes to stdlib functions.

### Work Stream 6: Benchmark Baseline Management

**Problem**: Single baseline file (`native-compiler-baselines.json`) captured 2026-02-24 with no automated refresh or staleness detection.

**Approach**:

1. **Staleness warning in CI**: Add a check in `native-gates.sh` that warns (not fails) if baseline is older than 30 days. Compare `captured_at` field to current date.
2. **Refresh script**: Create `scripts/refresh-baselines.sh` that runs the benchmark suite and overwrites the baseline with new captures + metadata (date, OS, CPU, rust version).
3. **Documentation**: Add a "Refreshing Baselines" section to `docs/native-regression-policy.md`.

## Scope Exclusions

- No `tonic gate` Rust command (future work)
- No migration of existing 88 test files to new helpers
- No new language features or parser changes
- No registry install support (already scoped out in install RFC)
- No self-hosting work
- No native backend architecture changes

## Priority Order

| Priority | Stream | Rationale |
|----------|--------|-----------|
| P0 | Install tests | Safety: 722 lines of untested filesystem mutation |
| P1 | Test helpers | Enabler: needed for install tests and future work |
| P1 | Error diagnostics (3a, 3b) | Impact: directly improves developer + LLM experience |
| P2 | Gate hardening | Maintenance: reduces tech debt incrementally |
| P2 | Stdlib docs | Impact: improves LLM comprehension |
| P3 | Error diagnostics (3c — JSON) | Nice-to-have: enables tooling integration |
| P3 | Benchmark management | Low risk: baselines work, just need staleness guard |
