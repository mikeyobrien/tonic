# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.2] - 2026-04-03

### Added
- GitHub release workflow and cross-platform install script.
- Json module with streaming JSON parsing in stdlib.
- Native `integer_parse` host dispatch for C backend parity.
- Cargo profile tuning, feature-gated deps, and release optimization.
- Install test coverage, test helpers, and gate hardening.

### Changed
- Removed LLVM backend from mainline.
- Consolidated `emit_header` and deduplicated `parse_choices`.
- Deduplicated `is_native_executable` into tests/common.

### Fixed
- C backend: expanded host parity, runtime for comprehensions, closure match bindings, list/tuple helpers, streamed stderr in `sys_run`, suppressed final value after stdout writes, restored builtin parity.
- Native: `sys_run` timeout parity, boolean negation helpers, host-call diagnostic parity.
- Cross-platform test corrections for macOS compatibility.
- Loop artifact cleanup and clippy/runtime fixes.

## [0.1.0-alpha.1] - 2026-02-25

### Added
- Native release gate sequence with strict regression policy for interpreter and compiled targets.
- Differential and LLVM catalog parity enforcement in CI.
- Release checklist documenting required artifacts and strict-pass release policy.
