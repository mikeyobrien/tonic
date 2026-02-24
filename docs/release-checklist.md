# Release Checklist

Use this checklist before cutting a version tag.

## 1) Preflight

- [ ] Working tree clean (`git status`)
- [ ] Version/changelog updated
- [ ] No open rollback-class native regression issues

## 2) Required Native Gates (blocking)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test`
- [ ] `./scripts/differential-enforce.sh`
- [ ] Native benchmark contract + strict policy:

```bash
TONIC_BENCH_ENFORCE=0 \
TONIC_BENCH_JSON_OUT=.tonic/release/native-compiler-summary.json \
TONIC_BENCH_MARKDOWN_OUT=.tonic/release/native-compiler-summary.md \
./scripts/bench-native-contract-enforce.sh

./scripts/native-regression-policy.sh \
  .tonic/release/native-compiler-summary.json --mode strict
```

Tag cut is blocked unless policy output is `verdict=pass`.

## 3) Artifact Publishing (required)

- [ ] Upload benchmark artifacts for the candidate:
  - `.tonic/release/native-compiler-summary.json`
  - `.tonic/release/native-compiler-summary.md`
- [ ] Ensure report includes Rust/Go comparison metadata and failure reasons (if any)
- [ ] Link artifacts in release notes/PR

## 4) Tag + Release

- [ ] Create annotated tag (`vX.Y.Z`)
- [ ] Push tag after all gates are green
- [ ] Confirm `release-native-benchmarks` workflow succeeded and artifacts are attached
