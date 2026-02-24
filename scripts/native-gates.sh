#!/usr/bin/env bash
set -euo pipefail

artifact_dir="${TONIC_NATIVE_ARTIFACT_DIR:-.tonic/native-gates}"
manifest="${TONIC_NATIVE_BENCH_MANIFEST:-benchmarks/native-compiler-suite.toml}"
summary_json="${TONIC_NATIVE_SUMMARY_JSON:-$artifact_dir/native-compiler-summary.json}"
summary_md="${TONIC_NATIVE_SUMMARY_MD:-$artifact_dir/native-compiler-summary.md}"

mkdir -p "$artifact_dir"

printf '%s\n' '[native-gates] cargo fmt --all -- --check'
cargo fmt --all -- --check

printf '%s\n' '[native-gates] cargo clippy --all-targets --all-features -- -D warnings'
cargo clippy --all-targets --all-features -- -D warnings

printf '%s\n' '[native-gates] cargo test'
cargo test

printf '%s\n' '[native-gates] scripts/differential-enforce.sh'
./scripts/differential-enforce.sh

printf '%s\n' '[native-gates] scripts/llvm-catalog-parity-enforce.sh'
./scripts/llvm-catalog-parity-enforce.sh

printf '%s\n' '[native-gates] scripts/bench-native-contract-enforce.sh'
TONIC_BENCH_JSON_OUT="$summary_json" \
TONIC_BENCH_MARKDOWN_OUT="$summary_md" \
TONIC_BENCH_ENFORCE=0 \
./scripts/bench-native-contract-enforce.sh "$manifest"

printf '%s\n' '[native-gates] scripts/native-regression-policy.sh --mode strict'
./scripts/native-regression-policy.sh "$summary_json" --mode strict

printf '[native-gates] complete. benchmark artifacts:\n  %s\n  %s\n' "$summary_json" "$summary_md"
