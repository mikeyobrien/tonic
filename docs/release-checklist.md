# Release Checklist

Use this checklist before cutting a version tag.

## 1) Preflight

- [ ] Working tree clean (`git status`)
- [ ] Version metadata set to the intended release (for alpha: `X.Y.Z-alpha.N`)
- [ ] `CHANGELOG.md` updated with matching heading (`## [X.Y.Z-alpha.N]`)
- [ ] No open rollback-class native regression issues

## 2) Required Release Gates (blocking)

- [ ] Run one-shot alpha readiness gate:

```bash
./scripts/release-alpha-readiness.sh --version X.Y.Z-alpha.N
```

This command enforces all blocking checks:
- clean git working tree
- changelog presence + version heading
- `./scripts/native-gates.sh` (fmt, clippy, tests, differential, LLVM parity, strict policy)
- required benchmark artifacts exist

Tag cut is blocked unless readiness output ends with `alpha-readiness: pass`.

## 3) Artifact Publishing (required)

- [ ] Upload benchmark artifacts for the candidate:
  - `.tonic/release/native-compiler-summary.json`
  - `.tonic/release/native-compiler-summary.md`
  - `.tonic/release/native-compiled-summary.json`
  - `.tonic/release/native-compiled-summary.md`
- [ ] Ensure report includes Rust/Go comparison metadata and failure reasons (if any)
- [ ] Link artifacts in release notes/PR

## 4) Tag + Release

- [ ] Create annotated tag (`vX.Y.Z-alpha.N`)
- [ ] Push tag after all gates are green
- [ ] Confirm `release-native-benchmarks` workflow succeeded and artifacts are attached
