# Research: `tonic install`

## Existing Codebase Conventions

### Directory layout
- `.tonic/deps/` — project-local dependency cache (populated by `DependencyResolver::sync()` in `src/deps.rs`)
- `.tonic/cache/` — project-local IR artifact cache (content-addressed, `src/cache.rs`)
- `.tonic/build/` — referenced in help text for compile output
- No global `~/.tonic/` directory exists yet

### Manifest schema (`src/manifest.rs`, `src/manifest_parse.rs`)
- `ProjectManifest { entry, dependencies, package }`
- `PackageMetadata { name, version, description, license, authors, repository, keywords }`
- `Dependencies { path, git, registry }` — registry resolution not yet implemented
- Parser uses `toml::Value` and handles inline tables; extensible to new sections
- No `[[bin]]` section in any existing tonic.toml

### CLI dispatch (`src/main.rs:308-337`)
Commands: `run`, `check`, `test`, `fmt`, `compile`, `cache` (placeholder), `repl`, `verify`, `deps`, `publish`, `docs`, `lsp`. No `install` or `uninstall`.

### Shim patterns in the wild
1. **tonic-loops `bin/miniloops`**: Shell script that resolves symlinks, discovers repo root via relative path from script location, calls `tonic run "$REPO_DIR" "$@"`.
2. **`.miniloop/pi-adapter`**: Minimal `exec tonic run '/absolute/path' pi-adapter "$@"`.
3. **tonic-loops `bin/test`**: Simple `tonic test test/ --fail-fast --timeout 10000 "$@"`.

Pattern: shims are shell scripts that delegate to `tonic run <project-dir> [subcommand] [args]`.

### Package registry protocol (`docs/package-registry-protocol.md`)
Defines publish/fetch API. Uses `[package]` metadata in tonic.toml. No mention of binary declaration — assumes `bin/` directory convention.

## Answers to Open Questions

### Q1: Cache location — `~/.tonic/packages/` vs `.tonic/deps/`

**Recommendation: `~/.tonic/packages/<name>/`**

Rationale:
- `.tonic/deps/` is project-scoped (lives inside a project working tree). Global installs must be user-scoped.
- `~/.cargo/bin/` + `~/.cargo/registry/` is the direct analogue. Tonic should use `~/.tonic/bin/` for shims and `~/.tonic/packages/<name>/` for cached source trees.
- For local-path installs, the source can be symlinked or copied. For git installs, clone into `~/.tonic/packages/<name>/`.
- Layout: `~/.tonic/packages/<name>/` contains the full project tree (with `tonic.toml`, `src/`, `bin/`, etc.).

### Q2: Shim format — shell script vs compiled stub

**Recommendation: Shell script (MVP), with option for compiled stubs later.**

Rationale:
- All existing shims are shell scripts. The pattern is proven and debuggable.
- Shell scripts have zero compilation overhead and instant modification.
- A compiled shim would only matter for startup latency (shell fork+exec vs direct exec). Since `tonic run` already forks a process, the shell overhead is negligible.
- Future: if `tonic compile` matures, `tonic install --compile` could produce native binaries instead of shims.

Shim template:
```sh
#!/bin/sh
set -eu
exec tonic run '~/.tonic/packages/<name>' "$@"
```

### Q3: Version/update management for git sources

**Recommendation: Pin to commit hash at install time. `tonic update <name>` pulls latest.**

- At install time from git: clone, resolve HEAD (or specified ref), record commit hash in `~/.tonic/packages.toml` (a global manifest).
- `tonic update <name>` re-fetches and updates to latest commit on the tracked ref.
- `tonic install <git-url>#<ref>` allows pinning to a branch/tag/commit.
- No semver solving — that's a registry concern for later.

### Q4: `[[bin]]` in tonic.toml vs `bin/` directory convention

**Recommendation: `bin/` directory convention is sufficient for MVP. Add `[[bin]]` later if needed.**

Rationale:
- Every existing tonic project uses the `bin/` convention. No project uses `[[bin]]`.
- The `bin/` convention is zero-config: drop a script in `bin/` and it's installable.
- `[[bin]]` would be needed if a project wants to declare binaries that don't have shell wrapper scripts (e.g., "install my `src/main.tn` as command `foo`"). That's a later concern.
- Discovery: `tonic install` scans `<project>/bin/` for executable files and installs each as a shim in `~/.tonic/bin/`.

However, consider a minimal extension: if `[project]` has a `name` field and no `bin/` directory, `tonic install` could create a shim named after the project that runs `tonic run <project-dir>`. This handles the common case of single-binary projects without requiring `bin/`.

### Q5: `tonic install .` (dev install from current directory)

**Recommendation: Symlink mode for local paths.**

- `tonic install .` or `tonic install ./path/to/project`: instead of copying source to `~/.tonic/packages/`, create a symlink `~/.tonic/packages/<name> → /absolute/path/to/project`.
- Shims then resolve through the symlink to the live project directory.
- This gives a `pip install -e .` / `npm link` style workflow: edit source, changes are immediately reflected.
- `tonic install --copy .` could force a snapshot copy if the user wants immutability.
- Track install mode (symlink vs copy vs git) in `~/.tonic/packages.toml`.

## Analogous Systems

| System | Install command | Global bin | Package cache | Shim strategy |
|--------|----------------|------------|---------------|---------------|
| Cargo | `cargo install` | `~/.cargo/bin/` | `~/.cargo/registry/` | Compiled binary |
| npm | `npm install -g` | `{prefix}/bin/` | `{prefix}/lib/node_modules/` | Shell/cmd shims |
| pip | `pip install` | `~/.local/bin/` | `~/.cache/pip/` | Entry-point scripts |
| Deno | `deno install` | `~/.deno/bin/` | `~/.cache/deno/` | Shell shim |
| Mix/Elixir | `mix escript.install` | `~/.mix/escripts/` | N/A | Compiled escript |

Tonic's model is closest to **Deno** and **npm**: shell shims pointing to cached source, executed by the runtime.

## Global Manifest: `~/.tonic/packages.toml`

Track installed packages globally:

```toml
[packages.miniloops]
source = "git"
url = "https://github.com/user/tonic-loops.git"
ref = "main"
commit = "abc123"
installed_bins = ["miniloops", "test"]
installed_at = "2026-03-27T12:00:00Z"

[packages.my-tool]
source = "path"
path = "/Users/rook/projects/my-tool"
symlink = true
installed_bins = ["my-tool"]
installed_at = "2026-03-27T12:00:00Z"
```

## Unanswered Questions

1. **PATH setup UX**: Should `tonic install` automatically modify shell profiles (~/.bashrc, ~/.zshrc, config.fish) to add `~/.tonic/bin/` to PATH, or just print instructions? Recommendation: print instructions only (less invasive, matches cargo's approach on first install).
2. **Conflict resolution**: What happens when two packages install a binary with the same name? Recommendation: warn and require `--force` to overwrite.
3. **Dependency resolution for installed packages**: If an installed package has `[dependencies]` in its tonic.toml, should `tonic install` run `tonic deps sync` in the cached package? Recommendation: yes, automatically.
