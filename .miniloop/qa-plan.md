# QA Plan — tonic install feature

## Domain
Rust CLI application (the Tonic language toolchain). Recent changes add three new subcommands: `install`, `uninstall`, `installed`.

## Recent Changes (HEAD~2)
- `src/cmd_install.rs` (new, 722 lines) — implements `handle_install`, `handle_uninstall`, `handle_installed` plus helpers: `discover_binaries`, `generate_shims`, `read_package_name`, `copy_dir_recursive`, manifest I/O, help text, and 9 unit tests.
- `src/main.rs` — wires `install`/`uninstall`/`installed` into the CLI dispatch and adds the `cmd_install` module.
- `src/cmd_deps.rs` — updates help text to list the three new commands.

## Available Validation Surfaces

| # | Surface | Tool | Status |
|---|---------|------|--------|
| 1 | **Compilation** (`cargo build`) | cargo | pending |
| 2 | **Unit tests** (`cargo test --lib` or `cargo test cmd_install`) | cargo | pending |
| 3 | **Clippy** (`cargo clippy`) | cargo | pending |
| 4 | **CLI help smoke test** (`tests/cli_help_smoke.rs`) — existing test checks help output for known commands; new commands may need coverage | cargo test | pending |
| 5 | **Code review: structural** — read-only inspection of wiring, error paths, security (path traversal, shim injection) | manual read | pending |
| 6 | **Shim correctness** — verify generated shim scripts are well-formed, safe, and use `set -eu` | manual read | pending |
| 7 | **Manifest round-trip** — unit test already exists (`packages_manifest_round_trip`), validate it runs | cargo test | pending |
| 8 | **Edge case: fallback shim without explicit name** — unit test exists (`discover_binaries_errors_without_explicit_name_and_no_bin_dir`), validate per recent fix `273af1e` | cargo test | pending |

## Ordered Validation Steps

1. `cargo build` — confirms the new module compiles and links.
2. `cargo test -p tonic cmd_install` — runs the 9 inline unit tests.
3. `cargo clippy -- -D warnings` — lint check on new code.
4. `cargo test --test cli_help_smoke` — verify help text lists new commands (may need update).
5. Structural code review of `cmd_install.rs` — error handling, security, correctness.
6. Shim content review — verify shim scripts are safe and correct.
