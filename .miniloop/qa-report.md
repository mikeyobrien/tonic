# QA Report — `tonic install` feature

**Date:** 2026-03-27
**Scope:** Recent changes on `tonic-install` branch (HEAD~2: commits `273af1e`, `9a0dbd3`)
**Domain:** Rust CLI — new `install`, `uninstall`, `installed` subcommands

## Summary

**5 of 6 validation surfaces passed. 1 failed (clippy).**

The new install feature compiles, passes all 9 unit tests, passes CLI smoke tests, and is structurally sound with no security concerns. Clippy reports 5 warnings in the new code that should be fixed before merge.

## Results

| # | Surface | Status | Evidence |
|---|---------|--------|----------|
| 1 | Compilation | **PASS** | `cargo build` succeeds |
| 2 | Unit tests | **PASS** | 9/9 tests pass (`cargo test cmd_install`) — covers manifest round-trip, binary discovery, shim generation, fallback edge cases |
| 3 | Clippy | **FAIL** | 5 warnings promoted to errors with `-D warnings` |
| 4 | CLI help smoke | **PASS** | `cargo test --test cli_help_smoke` succeeds |
| 5 | Structural review | **PASS** | Wiring correct, error paths sound, no path traversal or injection risks |
| 6 | Shim correctness | **PASS** | Well-formed `#!/bin/sh` + `set -eu` + `exec`, single-quoted paths, 0o755 perms |

## Failure Detail

### Surface 3 — Clippy (5 warnings in `src/cmd_install.rs`)

1. **Line 90** — `while let Some(arg) = iter.next()` → use `for arg in iter` (`while_let_on_iterator`)
2. **Line 142** — useless `format!()` on string literal → use `.to_string()` (`useless_format`)
3. **Lines 162-164** — useless `format!()` on string literal → use `.to_string()` (`useless_format`)
4. **Lines 334-336** — useless `format!()` on string literal → use `.to_string()` (`useless_format`)
5. **Line 526** — `print!("{}", "INSTALLED")` → `print!("INSTALLED")` (`print_literal`)

**Note:** Pre-existing clippy warnings also exist in `src/interop/bitwise_mod.rs` and `src/name_resolve.rs`; these are unrelated to the changes under test.

## Structural Observations (non-blocking)

- Wiring in `main.rs` correctly dispatches all three subcommands.
- All error paths return proper exit codes via `CliDiagnostic`.
- `bin_name` sourced from `read_dir` (not user input) — no path traversal risk.
- Shim `exec` uses single-quoted paths — safe against shell injection.
- `copy_dir_recursive` follows symlinks — acceptable for a local install tool.

## Verdict

**Not ready to merge.** The 5 clippy warnings are trivial to fix (mechanical changes, no logic impact). Once resolved, the feature is clean.
