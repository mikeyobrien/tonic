# Tonic Package Registry Protocol

This document describes the manifest format for registry-ready packages,
the planned registry API, version resolution strategy, and the intended
behaviour of `tonic publish`.

---

## 1. Manifest Format

### Full Example

```toml
[project]
entry = "src/main.tn"

[package]
name        = "my_lib"
version     = "0.2.0"
description = "A concise description of the library."
license     = "MIT"
repository  = "https://github.com/example/my_lib"
authors     = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
keywords    = ["json", "parsing", "tonic"]

[registries]
default = "https://registry.tonic-lang.org"

[dependencies]
# Registry dep – shorthand (uses [registries] default)
json      = "~> 1.0"

# Registry dep – table form with explicit registry override
http      = { version = "~> 0.5", registry = "https://registry.example.com" }

# Git dep (already supported)
utils     = { git = "https://github.com/example/utils.git", rev = "v0.1.0" }

# Path dep (already supported)
local_lib = { path = "../local_lib" }
```

### Field Reference

#### `[project]` section (required)

| Field   | Type   | Required | Description                     |
|---------|--------|----------|---------------------------------|
| `entry` | string | yes      | Relative path to the entry file |

#### `[package]` section (optional for local projects; required for publishing)

| Field         | Type            | Required for publish | Description                             |
|---------------|-----------------|----------------------|-----------------------------------------|
| `name`        | string          | yes                  | Package name (lowercase, underscores)   |
| `version`     | string (semver) | yes                  | Package version, e.g. `"1.2.3"`        |
| `description` | string          | yes                  | One-line description                    |
| `license`     | string          | no                   | SPDX license identifier, e.g. `"MIT"`  |
| `authors`     | array of string | no                   | Author names / emails                   |
| `repository`  | string          | no                   | URL to source repository                |
| `keywords`    | array of string | no                   | Up to five discovery keywords           |

#### `[registries]` section (optional)

| Field     | Type   | Description                                              |
|-----------|--------|----------------------------------------------------------|
| `default` | string | Base URL of the default registry used for version deps   |

When absent, the built-in default registry `https://registry.tonic-lang.org`
is used (once registry support is implemented).

#### `[dependencies]` entries

A dependency entry can take three forms:

```toml
# 1. Shorthand version string — registry dep using [registries] default
name = "~> 1.0"

# 2. Table with version — registry dep, optional registry override
name = { version = "~> 1.0", registry = "https://..." }

# 3. Git dep (existing)
name = { git = "https://...", rev = "abc123" }

# 4. Path dep (existing)
name = { path = "../relative/or/absolute" }
```

---

## 2. Backward Compatibility

Existing `tonic.toml` files without a `[package]` section continue to work
unchanged. The `[package]` section is always optional for local development
and `tonic run` / `tonic check` / `tonic test` / `tonic compile`.

---

## 3. Version Resolution Strategy

Tonic version requirements follow the Elixir / Hex convention:

| Operator  | Meaning                             | Example          |
|-----------|-------------------------------------|------------------|
| `~> X.Y`  | `>= X.Y` and `< X+1.0` (if Y == 0) | `~> 1.0`         |
| `~> X.Y`  | `>= X.Y` and `< X.(Y+1)`           | `~> 1.2`         |
| `~> X.Y.Z`| `>= X.Y.Z` and `< X.Y+1`           | `~> 1.2.3`       |
| `^X.Y`    | `>= X.Y` and `< X+1.0`             | `^2.1`           |
| `>= X.Y`  | At least X.Y                        | `>= 0.5`         |
| `== X.Y.Z`| Exact version pin                   | `== 1.0.0`       |

### Resolution Algorithm (planned)

1. Collect all version requirements for each package name across the
   dependency graph (direct + transitive).
2. Fetch the version index from the registry for each package.
3. Select the **highest version** that satisfies **all** constraints.
4. Fail with a conflict diagnostic if no satisfying version exists.
5. Write the resolved set to `tonic.lock` (extending the existing lockfile
   format with a `registry_deps` table).

The resolver will prefer:
- Stable releases over pre-releases unless a pre-release is explicitly required.
- The highest patch within an accepted minor range.

---

## 4. Planned Registry API

The registry exposes a REST API over HTTPS.

### Base URL

```
https://registry.tonic-lang.org/v1
```

### Endpoints

#### `GET /packages/{name}`

Returns the package index with all published versions.

```json
{
  "name": "json",
  "versions": [
    { "version": "1.0.0", "checksum": "sha256:abc..." },
    { "version": "1.1.0", "checksum": "sha256:def..." }
  ]
}
```

#### `GET /packages/{name}/{version}`

Returns metadata for a specific version.

```json
{
  "name": "json",
  "version": "1.1.0",
  "description": "JSON parsing library for Tonic",
  "license": "MIT",
  "checksum": "sha256:def...",
  "dependencies": {
    "utf8": "~> 0.2"
  },
  "download_url": "https://registry.tonic-lang.org/v1/packages/json/1.1.0/download"
}
```

#### `GET /packages/{name}/{version}/download`

Downloads the package tarball (`.tar.gz`).

#### `POST /packages`

Publishes a new package version. Requires authentication via API token.

Request body: `multipart/form-data` with:
- `manifest`: the `tonic.toml` contents
- `tarball`: gzipped tar archive of source files

### Package Naming Conventions

- Lowercase letters, digits, and underscores only (`[a-z0-9_]+`).
- Must start with a letter.
- Maximum 64 characters.
- Examples: `json`, `http_client`, `uuid_v4`.

### Authentication

API tokens are scoped per-user and passed via `Authorization: Bearer <token>`.

---

## 5. `tonic publish` Behaviour (when implemented)

When `tonic publish` is fully implemented it will:

1. Load `tonic.toml` from the current project root.
2. Validate that `[package]` contains `name`, `version`, and `description`.
3. Validate the version string is valid semver.
4. Run `tonic check` to ensure the package compiles cleanly.
5. Bundle all `.tn` source files (excluding `target/` and `.tonic/`) into a
   `.tar.gz` archive.
6. Compute a `sha256` checksum of the archive.
7. Upload the archive to the configured registry via `POST /packages`.
8. Print the published package URL on success.

**Current status:** The command parses arguments and validates manifest fields,
then prints `Publishing to registry is not yet supported` and exits 0.

---

## 6. `tonic.lock` Extension (planned)

When registry dependencies are supported, `tonic.lock` will gain a new table:

```toml
version = 1

[registry_deps.json]
version  = "1.1.0"
checksum = "sha256:def..."
url      = "https://registry.tonic-lang.org/v1/packages/json/1.1.0/download"

[git_deps.utils]
url = "https://github.com/example/utils.git"
rev = "abc123"
```

The lockfile format is versioned; future additions will increment `version`.
