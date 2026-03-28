# QA Progress

## Surfaces

| # | Surface | Command / Method | Status |
|---|---------|-----------------|--------|
| 1 | Compilation | `cargo build` | passed |
| 2 | Unit tests (cmd_install) | `cargo test cmd_install` | passed |
| 3 | Clippy | `cargo clippy -- -D warnings` | failed |
| 4 | CLI help smoke test | `cargo test --test cli_help_smoke` | passed |
| 5 | Structural code review | read-only: wiring, error paths, security | passed |
| 6 | Shim correctness review | read-only: shim script content | passed |

## Execution Order
1 → 2 → 3 → 4 → 5 → 6 (sequential; early failure in 1 blocks 2-4)

## Notes
- Surface 2 covers qa-plan items 7 (manifest round-trip) and 8 (fallback shim edge case) since those are inline unit tests in cmd_install.
- Surfaces 5-6 are read-only structural inspections with specific evidence queries.

## Results Detail

### Surface 3 — Clippy failures in cmd_install.rs

5 warnings (all `-D warnings` promoted to errors):

1. **Line 90**: `while let Some(arg) = iter.next()` → should be `for arg in iter` (`while_let_on_iterator`)
2. **Line 142**: useless `format!("registry install not yet supported...")` → use `.to_string()` (`useless_format`)
3. **Lines 162-164**: useless `format!("path does not appear to be a tonic project...")` → use `.to_string()` (`useless_format`)
4. **Lines 334-336**: useless `format!("no installable binaries found...")` → use `.to_string()` (`useless_format`)
5. **Line 526**: print literal `"INSTALLED"` can be inlined into format string (`print_literal`)

Note: pre-existing clippy warnings also exist in `src/interop/bitwise_mod.rs` and `src/name_resolve.rs` (not related to recent changes).

### Surface 5 — Structural review (passed)

- Wiring in main.rs correct: dispatches install/uninstall/installed to handlers.
- All error paths return proper exit codes via CliDiagnostic.
- No path traversal risk: bin_name comes from `read_dir`, not user input.
- Shim `exec` uses single-quoted paths — safe against shell injection.
- `copy_dir_recursive` follows symlinks (acceptable for local install tool).

### Surface 6 — Shim correctness (passed)

- Shims: `#!/bin/sh`, `set -eu`, `exec` — correct pattern.
- Single-quoted paths prevent injection.
- Both bin/-delegating and fallback (`tonic run`) shim variants well-formed.
- Permissions 0o755 on Unix — correct.

## Current Step
QA report compiled at `.miniloop/qa-report.md`. All 6 surfaces fully validated; clippy is the sole failure with 5 mechanical warnings fully characterized above.

## Next Role / Next Action
inspector (routed from qa.failed) — All surfaces have been inspected and executed. No new surfaces to discover. The 5 clippy warnings are fully documented in the QA report and require fixes outside this QA loop. Expected action: emit `task.complete` with the unresolved clippy gaps summary.
