# Context

## Objective
Implement `tonic install`, `tonic uninstall`, and `tonic installed` CLI subcommands per the RFC at `docs/rfcs/tonic-install.md` and the code task at `.agents/tasks/tonic-install/tonic-install.code-task.md`.

## Source task
- Code task: `.agents/tasks/tonic-install/tonic-install.code-task.md`
- RFC: `docs/rfcs/tonic-install.md`

## Existing implementation facts
- CLI dispatch: `src/main.rs` — `run()` matches subcommand strings, delegates to `handle_*` functions
- Command handler pattern: `src/cmd_deps.rs` — `handle_deps(args: Vec<String>) -> i32`, uses `CliDiagnostic` for errors
- Module wiring: `#[path = "cmd_*.rs"] mod cmd_*; use cmd_*::*;` at bottom of `main.rs`
- Error handling: `src/cli_diag.rs` — `CliDiagnostic::failure()`, `::usage_with_hint()`, `EXIT_OK/EXIT_FAILURE/EXIT_USAGE`
- Manifest: `src/manifest.rs` + `src/manifest_parse.rs` — `load_project_manifest(path) -> Result<ProjectManifest, String>`, `PackageMetadata { name, ... }`
- Dependency sync: `src/deps.rs` — `DependencyResolver::sync(&deps, &root) -> Result<Lockfile, String>`
- `toml` crate already in dependencies
- Help text functions: `print_*_help()` in `src/cmd_deps.rs`
- Tests: `src/main.rs` has `mod tests` with CLI dispatch tests

## Directory layout target
```
~/.tonic/
├── bin/           # shims
├── packages/      # cached sources (symlinks or clones)
└── packages.toml  # global manifest
```

## Key design decisions from RFC
- Local path: symlink by default, `--copy` flag copies
- Git URL: `git clone`, support `#ref` suffix
- Registry name: error "not yet supported"
- Shim: `#!/bin/sh` + `exec` passthrough
- Binary discovery: scan `bin/` dir, fallback to single shim via `tonic run`
- Conflict: error on binary name collision unless `--force`
- Reinstall: update in place
- PATH setup: print instructions on first install, no auto-modification
