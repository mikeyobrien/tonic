#!/usr/bin/env bash
set -euo pipefail

printf 'Running differential correctness gate...\n'
cargo test --test differential_backends -- --nocapture
