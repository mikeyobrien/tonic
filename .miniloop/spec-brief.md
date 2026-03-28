# Spec Brief: `tonic install`

**Title:** tonic install
**Slug:** tonic-install
**Output paths:**
- RFC: `docs/rfcs/tonic-install.md`
- Code task: `.agents/tasks/tonic-install/tonic-install.code-task.md`

## Objective

Add a `tonic install` CLI subcommand that fetches/resolves a tonic module (from a local path, git URL, or eventually the registry) and places executable shims on the user's PATH so the module's binaries are globally available.

## Motivating Example

Today, miniloops (`tonic-loops`) exposes `bin/miniloops` — a shell wrapper that resolves its repo root and calls `tonic run "$REPO_DIR" "$@"`. To use it, you must either:
- Manually symlink `bin/miniloops` into your PATH, or
- Use full paths like `./.miniloop/miniloops`.

`tonic install tonic-loops` (or `tonic install ./path/to/tonic-loops`) should make `miniloops` available as a first-class command.

## Goals

1. **`tonic install <source>`** resolves and caches a tonic project, then installs PATH-available shims for each binary it declares.
2. **Sources:** local path (MVP), git URL, registry name (future).
3. **Convention:** a tonic project declares binaries via a `bin/` directory (existing convention in tonic-loops) or via `[[bin]]` entries in `tonic.toml`.
4. **Global bin directory:** `~/.tonic/bin/` — user adds this to PATH once.
5. **Shim strategy:** installed shims call `tonic run <cached-project-path> "$@"`, similar to the existing `bin/miniloops` pattern.
6. **`tonic uninstall <name>`** removes shims and optionally cached sources.
7. **`tonic install --list`** shows installed modules.

## Non-Goals

- Native/compiled binary installation (future optimization; shims via `tonic run` are sufficient for now).
- Version resolution or semver constraint solving (registry deps aren't implemented yet).
- Sandboxing or permission scoping for installed modules.
- Windows support (follow existing Unix-first posture).

## Constraints

- Must not break existing `tonic deps` or `tonic.toml` dependency workflows.
- `install` is a new subcommand — no existing placeholder to replace.
- The `cache` subcommand is a placeholder; install may share cache infrastructure but should not conflict.
- Shims must resolve symlinks correctly (the existing `bin/miniloops` pattern already handles this).

## Assumptions

- `~/.tonic/bin/` is an acceptable global bin path (analogous to `~/.cargo/bin/`).
- The `bin/` directory convention in tonic projects is the primary way to declare installable executables.
- `tonic run <project-dir>` remains the execution model for installed modules (no compilation step needed).

## Open Questions (for researcher)

1. Should `tonic install` cache the project source under `~/.tonic/packages/` or reuse `.tonic/deps/`?
2. Should the shim be a shell script or a compiled binary stub?
3. How should version/update management work for git-sourced installs?
4. Should `tonic.toml` gain a `[[bin]]` section, or is the `bin/` directory convention sufficient?
5. How does `tonic install .` (install from current directory) interact with development workflows?
