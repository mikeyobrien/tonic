# Plan

## Active slice
Slice 1: Scaffold `cmd_install.rs` with all three handlers, wire dispatch in `main.rs`, implement local-path install end-to-end (symlink + shim generation + packages.toml + help text).

## Why this slice first
Local-path install exercises the full lifecycle (resolve → cache → discover bins → generate shims → write manifest → print summary). Once this works, git URL support and uninstall/installed are incremental additions on a proven foundation.

## Slice breakdown

### Slice 1 — Local-path install (active)
- Create `src/cmd_install.rs` with `handle_install`, `handle_uninstall`, `handle_installed`
- Wire dispatch in `src/main.rs` for `"install"`, `"uninstall"`, `"installed"`
- Implement `handle_install` for local-path source:
  - Parse args (`--copy`, `--force`, `-h`/`--help`)
  - Resolve local path → canonicalize, verify `tonic.toml` exists
  - Read package name from manifest (fallback: dir name)
  - Create `~/.tonic/{bin,packages}/` dirs
  - Symlink (or copy with `--copy`) source → `~/.tonic/packages/<name>/`
  - Discover binaries from `bin/` dir (or fallback shim)
  - Check for binary name conflicts in `packages.toml`
  - Generate shims in `~/.tonic/bin/`
  - Write/update `packages.toml`
  - Print summary + PATH instructions on first install
- Implement help text for all three commands
- Stub `handle_uninstall` and `handle_installed` (just help + "not yet implemented" or minimal)
- Update `print_help()` in main.rs to list new commands
- Add unit tests for the new dispatch arms

### Slice 2 — Uninstall + installed listing
- Implement `handle_uninstall`: read manifest, remove shims, remove package dir, update manifest
- Implement `handle_installed`: read manifest, print table
- Tests

### Slice 3 — Git URL install
- Source detection (URL vs path vs registry name)
- `git clone` into `~/.tonic/packages/<name>/`
- `#ref` suffix parsing and checkout
- Record commit hash in manifest
- Dependency sync after clone
- Tests

### Slice 4 — Edge cases + polish
- Registry name error message
- `--force` for binary conflicts
- Reinstall (update in place)
- No-binaries error
- Comprehensive error messages per RFC

## Builder checklist for Slice 1
- [ ] Create `src/cmd_install.rs`
- [ ] Wire dispatch in `src/main.rs` (match arms + module declaration)
- [ ] Implement local-path install flow
- [ ] Implement packages.toml read/write
- [ ] Implement shim generation
- [ ] Implement binary discovery
- [ ] Implement conflict detection
- [ ] Add help text for install/uninstall/installed
- [ ] Update `print_help()` with new commands
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (existing + new tests)

## Verification plan
1. `cargo build` — compiles cleanly
2. `cargo test` — all existing tests pass, new dispatch tests pass
3. Manual: `cargo run --bin tonic -- install --help` shows help
4. Manual: create a test project with `tonic.toml` and `bin/` script, install it, verify shim and manifest
5. Manual: verify `tonic --help` lists new commands

## Suggested verification commands
- `cargo build 2>&1`
- `cargo test 2>&1`
- `cargo run --bin tonic -- --help`
- `cargo run --bin tonic -- install --help`

## Explicitly out of scope
- Windows support
- `tonic update` command
- `[[bin]]` manifest section
- Compiled binary optimization
