#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/observability.sh
source "$script_dir/lib/observability.sh"
tonic_obs_script_init "differential-enforce" "$@"
trap 'tonic_obs_finish "$?"' EXIT

printf 'Running differential correctness gate...\n'
tonic_obs_run_step 'cargo test --test differential_backends -- --nocapture' \
  cargo test --test differential_backends -- --nocapture
