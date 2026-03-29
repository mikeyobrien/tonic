# RFC: Build Speed & Release Pipeline

## Summary

Six targeted improvements to Tonic build speed, CI efficiency, and release automation: (1) Cargo profile tuning & feature-gated dependencies, (2) dev/CI observability & flag parity, (3) integration test binary consolidation, (4) benchmark binary extraction to workspace members, (5) CI artifact passing to eliminate redundant builds, (6) release profile optimization & automated release pipeline via cargo-dist.

## Motivation

The Tonic compiler has a growing codebase (254 crates in `Cargo.lock`, 177 integration test files, 16+ direct dependencies) but no build-speed or release-automation investment. Several compounding inefficiencies slow the inner dev loop and CI pipeline:

- **No profile tuning.** `Cargo.toml` has no `[profile.dev]` or `[profile.test]` — full DWARF debug info (`debug=2`) inflates link times, and macOS `dsymutil` passes add seconds to every incremental build.
- **177 test binaries.** Each of the 177 `tests/*.rs` files compiles as a separate binary linked against the full dependency tree. A clean `cargo test` links 177 binaries × 2-5s each.
- **Unconditional heavy dependencies.** `tower-lsp`, `tokio`, `reqwest`, and `ed25519-dalek` are compiled for every target despite being used in only 7 source files. Tests never import them (subprocess-based via `assert_cmd`), yet every `cargo test` compiles their full transitive closure.
- **CI rebuilds from source 4×.** `native-gates.yml` has 4 jobs, each independently restoring cache, installing toolchain, and rebuilding — no artifact passing between the build job and 3 downstream jobs.
- **Benchmark binaries carry full dep tree.** `benchsuite` in `src/bin/` uses only `serde`/`serde_json`/`toml`/`std` but compiles against all 16+ crate dependencies.
- **Manual release process.** No git tagging, GitHub Release creation, cross-platform builds, or `cargo publish` automation. Single-platform (Linux) only.
- **Dev/CI flag divergence.** `devenv.nix` test alias runs bare `cargo test` while CI runs `cargo test --all-features` — a silent correctness gap.

These are distinct from the `tonic-dev-toolchain` RFC (which covers test coverage, diagnostics, and gate hardening). This RFC targets build speed and release infrastructure.

**Provenance:** All 6 ideas validated in the autoideas scan (step-1 ideas-report, chain `chain-mnamyuig-2smw`).

## Design

### Work Stream 1: Cargo Profile Tuning & Feature-Gated Dependencies (P0)

**Problem**: No `[profile.dev]` or `[profile.test]`; no `[features]` section. Full debug info and all heavy deps compiled unconditionally.

**Approach — Phase A: Profile Tuning (Low Effort)**

Add to `Cargo.toml`:

```toml
[profile.dev]
debug = 1                      # line-tables-only — ~50% smaller debuginfo
split-debuginfo = "unpacked"   # macOS: skip dsymutil bottleneck

[profile.test]
opt-level = 1                  # faster test execution at minimal compile cost
debug = 0                      # tests rarely need debuginfo
```

**Approach — Phase B: Feature-Gated Dependencies (Medium Effort)**

Define two Cargo features:

```toml
[features]
default = ["lsp", "network"]
lsp = ["dep:tower-lsp", "dep:tokio"]
network = ["dep:reqwest", "dep:ed25519-dalek"]
```

Guard source files with `#[cfg(feature = "lsp")]` / `#[cfg(feature = "network")]`. The LSP subsystem (5 files in `src/lsp/`) and network subsystem (`src/interop/system.rs`, `src/interop/system_http.rs`) are isolated — no cross-feature imports.

- `cargo test --no-default-features` skips 30-50% of the transitive dependency tree
- `cargo build` (default) still produces the full binary
- `devenv.nix` test script changes to `cargo test --no-default-features` since tests are subprocess-based

**Risk**: `#[cfg(feature)]` on entire modules requires audit of import chains. `rustyline` (REPL) may have cross-feature boundaries.

### Work Stream 2: Dev/CI Observability & Flag Parity (P0)

**Problem**: Three divergences between local dev and CI environments.

**Approach — Fix A: Flag Parity (Trivial)**

Change `devenv.nix` `test.exec` from `cargo test` to `cargo test --all-features` to match CI's `native-gates.sh`. One-line fix.

**Approach — Fix B: Observability Wrapper (Low Effort)**

Wrap devenv aliases to optionally source the existing observability library (`scripts/lib/observability.sh`):

```nix
scripts.test.exec = ''
  if [ -n "''${TONIC_OBS_ENABLE:-}" ]; then
    source ./scripts/observability.sh
    tonic_obs_run_step "dev-test" cargo test --all-features
  else
    cargo test --all-features
  fi
'';
```

Default behavior unchanged. `TONIC_OBS_ENABLE=1` gives local timing parity with CI.

**Approach — Fix C: Nix `packages.default` (Low Effort)**

Add to `flake.nix`:

```nix
packages.default = pkgs.rustPlatform.buildRustPackage {
  pname = "tonic";
  version = "0.4.0";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
};
```

Enables `nix build`, `nix run`, and Nix binary cache integration.

**Risk**: `buildRustPackage` needs `cargoHash` or `cargoLock.lockFile`; workspace layout changes (Work Stream 4) will affect the Nix derivation.

### Work Stream 3: Integration Test Binary Consolidation (P1)

**Problem**: 177 test files → 177 binaries. Each links against the full dependency tree (2-5s per link).

**Approach**: Consolidate from 177 binaries to ~12 using `[[test]]` entries in `Cargo.toml`, grouping by natural filename prefix:

| Group | Files | Becomes |
|-------|-------|---------|
| `check_*` | 48 | `tests/check/mod.rs` |
| `run_*` | 47 | `tests/run/mod.rs` |
| `runtime_*` | 20 | `tests/runtime/mod.rs` |
| `compile_*` | 10 | `tests/compile/mod.rs` |
| `system_*` | 9 | `tests/system/mod.rs` |
| `install_*` | 7 | `tests/install/mod.rs` |
| `verify_*` | 7 | `tests/verify/mod.rs` |
| `cli_*` | 6 | `tests/cli/mod.rs` |
| Other | ~20 | `tests/misc/mod.rs` or 3-4 smaller groups |

Each group gets a `[[test]]` entry:

```toml
[[test]]
name = "check"
path = "tests/check/mod.rs"
```

Filtering remains: `cargo test --test check check_dump_ast`.

**CI script impact**: `differential-enforce.sh` targets `--test differential_backends` specifically — this test must either stay as a standalone binary or be placed in a known group. `native-gates.sh` references bare `cargo test` and needs no change (all groups included by default).

**Risk**: Must preserve `cargo test --test <name> <filter>` CLI used by CI scripts.

### Work Stream 4: Benchmark Binary Extraction to Workspace (P1)

**Problem**: `benchsuite` (9 files) in `src/bin/` uses only `serde`/`serde_json`/`toml`/`std` but compiles against the full 16+ crate dependency tree.

**Approach**: Convert to a Cargo workspace and extract the benchmark binary:

```
Cargo.toml          # [workspace] members = [".", "tools/benchsuite"]
tools/
  benchsuite/
    Cargo.toml      # deps: serde, serde_json, toml only
    src/main.rs     # + 8 submodules moved from src/bin/benchsuite/
```

Root `Cargo.toml` becomes a workspace root. The main `tonic` crate stays at `.` as a workspace member. `Cargo.lock` is shared.

**CI script updates**: `cargo build --release --bin benchsuite` → `cargo build --release -p benchsuite`. `cargo clippy --all-targets` → `cargo clippy --workspace`.

**Risk**: Workspace conversion changes `cargo clippy --all-targets` semantics. CI scripts and `devenv.nix` need updating. Nix derivation (Work Stream 2C) needs workspace awareness.

### Work Stream 5: CI Artifact Passing (P2)

**Problem**: 4 CI jobs each independently restore cache, install toolchain, and rebuild from source despite `needs:` dependencies.

**Approach**: Restructure `native-gates.yml` into build-then-fan-out:

1. **Build job**: Compile, run `cargo test`, `cargo fmt --check`, `cargo clippy`, then `upload-artifact` the release binaries (`tonic`, `benchsuite`) and test artifacts.
2. **Downstream jobs**: `download-artifact` instead of rebuilding. Skip toolchain install and cache restore. Run only their specific gate scripts against pre-built binaries.
3. **Remove duplicate test run**: Job 1 runs `cargo test` which includes differential tests. `differential-enforce.sh` (Job 2) can be merged into Job 1 or reduced to differential-specific non-test checks.

```yaml
- name: Upload build artifacts
  uses: actions/upload-artifact@v4
  with:
    name: build-${{ github.sha }}
    path: |
      target/release/tonic
      target/release/benchsuite
```

**Risk**: GitHub Actions artifact upload/download adds ~30s overhead per job — must weigh against rebuild savings (~2-5 min per job). Net positive expected.

### Work Stream 6: Release Profile Optimization & Automated Release Pipeline (P2)

**Problem**: Release profile uses `lto = "thin"` and `codegen-units = 8` — suboptimal. No release automation (no tagging, no GitHub Releases, no cross-platform builds, no `cargo publish`).

**Approach — Phase A: Release Profile (Trivial)**

```toml
[profile.release]
lto = "fat"           # was "thin" — full LTO for max binary optimization
codegen-units = 1     # was 8 — single codegen unit for cross-module optimization
panic = "abort"       # already set
strip = "symbols"     # already set
```

Trade: ~2-3× longer release compile time for 10-20% smaller binary and 5-15% faster hot paths. Acceptable for infrequent release builds.

**Approach — Phase B: cargo-dist Integration (Medium Effort)**

Run `cargo dist init` to generate a release workflow:

- Triggers on git tag push (`v0.x.0`)
- Build matrix: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- Creates GitHub Release with platform-specific binaries
- Extracts changelog entries for release body
- Optionally runs `cargo publish`

`release-alpha-readiness.sh` becomes a pre-tag validation step. `release-native-benchmarks.yml` can be retired or repurposed.

**Risk**: `cargo-dist` requires `[workspace.metadata.dist]` or `[package.metadata.dist]` — may interact with workspace conversion (Work Stream 4). Coordinate ordering.

## Scope Exclusions

- No implementation code in this RFC — planning artifacts only
- No changes to the `tonic-dev-toolchain` RFC or its work streams
- No new language features, parser changes, or native backend architecture changes
- No registry install support or self-hosting work
- No full CI pipeline rewrite — incremental improvements only
- No migration of existing test code to new helpers (that's covered by `tonic-dev-toolchain`)

## Dependency Ordering

| Phase | Stream | Rationale |
|-------|--------|-----------|
| Phase 1 | WS1: Profile tuning (1A) | Standalone, no deps, immediate benefit |
| Phase 1 | WS2: Dev/CI parity (2A, 2B) | Standalone, trivial, fixes correctness gap |
| Phase 2 | WS1: Feature gating (1B) | Requires import chain audit; profile tuning should land first |
| Phase 2 | WS3: Test consolidation | Depends on understanding CI script test targets; benefits from profile tuning |
| Phase 3 | WS4: Workspace extraction | Depends on profile tuning being settled; changes project structure |
| Phase 3 | WS2: Nix packages.default (2C) | Should coordinate with workspace structure |
| Phase 4 | WS5: CI artifact passing | Depends on workspace structure being finalized |
| Phase 4 | WS6: Release pipeline | Depends on workspace structure; cargo-dist needs stable metadata |

## Priority Order

| Priority | Stream | Rationale |
|----------|--------|-----------|
| P0 | Profile tuning (1A) | Immediate, zero-risk, measurable link time improvement |
| P0 | Dev/CI flag parity (2A) | Trivial one-line fix, eliminates silent correctness gap |
| P0 | Observability wrapper (2B) | Low effort, leverages existing infrastructure |
| P1 | Feature gating (1B) | High impact (30-50% fewer deps in test), medium effort |
| P1 | Test consolidation (3) | High impact (177→12 binaries), medium effort |
| P1 | Workspace extraction (4) | Medium impact, enables parallel compilation |
| P2 | CI artifact passing (5) | High CI savings, depends on workspace finalization |
| P2 | Release profile (6A) | Trivial change, measurable binary improvement |
| P2 | Release pipeline (6B) | Medium effort, high long-term value |
| P2 | Nix packages.default (2C) | Low effort, enables reproducible builds |
