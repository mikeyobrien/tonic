#!/usr/bin/env bash

if [[ -n "${TONIC_OBS_LIB_LOADED:-}" ]]; then
  return 0
fi
TONIC_OBS_LIB_LOADED=1

TONIC_OBS_HELPER_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
TONIC_OBS_HELPER_PY="$TONIC_OBS_HELPER_DIR/observability_event.py"
TONIC_OBS_ACTIVE=0
TONIC_OBS_RUN_ID="${TONIC_OBS_RUN_ID:-}"
TONIC_OBS_TASK_ID="${TONIC_OBS_TASK_ID:-}"
TONIC_OBS_PARENT_RUN_ID="${TONIC_OBS_PARENT_RUN_ID:-}"
TONIC_OBS_DIR="${TONIC_OBS_DIR:-${PWD}/.tonic/observability}"
TONIC_OBS_TOOL_NAME=""
TONIC_OBS_COMMAND=""

_tonic_obs_now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

_tonic_obs_generate_id() {
  local prefix="$1"
  local stamp rand
  stamp="$(date -u +%Y%m%d_%H%M%S)"
  rand=$(printf '%06x' "$(( ((RANDOM << 1) ^ RANDOM) & 0xFFFFFF ))")
  printf '%s_%s_%s\n' "$prefix" "$stamp" "$rand"
}

_tonic_obs_warn() {
  printf 'observability warning: %s\n' "$1" >&2
}

_tonic_obs_call_python() {
  python3 "$TONIC_OBS_HELPER_PY" "$@"
}

_tonic_obs_safe_python() {
  local output status
  if output="$(_tonic_obs_call_python "$@" 2>&1)"; then
    printf '%s' "$output"
    return 0
  fi
  status=$?
  _tonic_obs_warn "$output"
  return "$status"
}

_tonic_obs_summary_exists() {
  [[ -f "$TONIC_OBS_DIR/runs/$1/summary.json" ]]
}

_tonic_obs_set_start_values() {
  local line key value
  while IFS='=' read -r key value; do
    case "$key" in
      run_id) TONIC_OBS_RUN_ID="$value" ;;
      task_id) TONIC_OBS_TASK_ID="$value" ;;
      parent_run_id) TONIC_OBS_PARENT_RUN_ID="$value" ;;
      output_root) TONIC_OBS_DIR="$value" ;;
    esac
  done <<< "$1"
}

_tonic_obs_maybe_synthesize_child_run() {
  local child_run_id="$1"
  local step_name="$2"
  local started_at="$3"
  local elapsed_ms="$4"
  local exit_code="$5"
  shift 5
  local cmd=("$@")

  if _tonic_obs_summary_exists "$child_run_id"; then
    return 0
  fi

  local status_text="ok"
  if [[ "$exit_code" -ne 0 ]]; then
    status_text="error"
  fi

  local start_output
  local start_args=(
    start-run
    --output-root "$TONIC_OBS_DIR"
    --tool-kind "script-step"
    --tool-name "$TONIC_OBS_TOOL_NAME"
    --command "$step_name"
    --cwd "$PWD"
    --worktree-root "$PWD"
    --started-at "$started_at"
    --run-id "$child_run_id"
    --task-id "$TONIC_OBS_TASK_ID"
    --parent-run-id "$TONIC_OBS_RUN_ID"
  )
  for arg in "${cmd[@]}"; do
    start_args+=("--argv-item=$arg")
  done
  if ! start_output="$(_tonic_obs_safe_python "${start_args[@]}")"; then
    return 0
  fi

  if ! _tonic_obs_safe_python finish-run \
    --output-root "$TONIC_OBS_DIR" \
    --run-id "$child_run_id" \
    --status "$status_text" \
    --exit-code "$exit_code" \
    --phase-name "$step_name" \
    --phase-status "$status_text" \
    --phase-elapsed-ms "$elapsed_ms" >/dev/null; then
    return 0
  fi
}

tonic_obs_script_init() {
  TONIC_OBS_COMMAND="$1"
  shift
  TONIC_OBS_TOOL_NAME="$(basename -- "$0")"
  TONIC_OBS_DIR="${TONIC_OBS_DIR:-${PWD}/.tonic/observability}"

  if [[ "${TONIC_OBS_ENABLE:-0}" != "1" ]]; then
    TONIC_OBS_ACTIVE=0
    return 0
  fi

  if [[ ! -f "$TONIC_OBS_HELPER_PY" ]]; then
    _tonic_obs_warn "missing helper: $TONIC_OBS_HELPER_PY"
    TONIC_OBS_ACTIVE=0
    return 0
  fi

  local output
  local init_args=(
    start-run
    --output-root "$TONIC_OBS_DIR"
    --tool-kind "script"
    --tool-name "$TONIC_OBS_TOOL_NAME"
    --command "$TONIC_OBS_COMMAND"
    --cwd "$PWD"
    --worktree-root "$PWD"
    --run-id "${TONIC_OBS_RUN_ID:-}"
    --task-id "${TONIC_OBS_TASK_ID:-}"
    --parent-run-id "${TONIC_OBS_PARENT_RUN_ID:-}"
  )
  for arg in "$@"; do
    init_args+=("--argv-item=$arg")
  done
  if ! output="$(_tonic_obs_safe_python "${init_args[@]}")"; then
    TONIC_OBS_ACTIVE=0
    return 0
  fi

  _tonic_obs_set_start_values "$output"
  export TONIC_OBS_DIR TONIC_OBS_RUN_ID TONIC_OBS_TASK_ID TONIC_OBS_PARENT_RUN_ID
  TONIC_OBS_ACTIVE=1
}

tonic_obs_finish() {
  local exit_code="${1:-0}"
  if [[ "$TONIC_OBS_ACTIVE" != "1" ]]; then
    return 0
  fi

  local status_text="ok"
  if [[ "$exit_code" -ne 0 ]]; then
    status_text="error"
  fi

  _tonic_obs_safe_python finish-run \
    --output-root "$TONIC_OBS_DIR" \
    --run-id "$TONIC_OBS_RUN_ID" \
    --status "$status_text" \
    --exit-code "$exit_code" >/dev/null || true
}

tonic_obs_record_artifact() {
  local kind="$1"
  local path="$2"
  if [[ "$TONIC_OBS_ACTIVE" != "1" ]]; then
    return 0
  fi

  _tonic_obs_safe_python record-artifact \
    --output-root "$TONIC_OBS_DIR" \
    --run-id "$TONIC_OBS_RUN_ID" \
    --kind "$kind" \
    --path "$path" >/dev/null || true
}

tonic_obs_run_step() {
  local step_name="$1"
  shift
  local cmd=("$@")

  if [[ "$TONIC_OBS_ACTIVE" != "1" ]]; then
    "${cmd[@]}"
    return "$?"
  fi

  local child_run_id started_at start_ns end_ns elapsed_ms exit_code status_text
  child_run_id="$(_tonic_obs_generate_id run)"
  started_at="$(_tonic_obs_now)"
  start_ns="$(date +%s%N)"

  _tonic_obs_safe_python start-step \
    --output-root "$TONIC_OBS_DIR" \
    --run-id "$TONIC_OBS_RUN_ID" \
    --step "$step_name" \
    --child-run-id "$child_run_id" \
    --command "$step_name" \
    --at "$started_at" >/dev/null || true

  if env \
    TONIC_OBS_ENABLE="${TONIC_OBS_ENABLE:-1}" \
    TONIC_OBS_DIR="$TONIC_OBS_DIR" \
    TONIC_OBS_RUN_ID="$child_run_id" \
    TONIC_OBS_TASK_ID="$TONIC_OBS_TASK_ID" \
    TONIC_OBS_PARENT_RUN_ID="$TONIC_OBS_RUN_ID" \
    "${cmd[@]}"; then
    exit_code=0
  else
    exit_code=$?
  fi

  end_ns="$(date +%s%N)"
  elapsed_ms=$(python3 - "$start_ns" "$end_ns" <<'PY'
import sys
start_ns = int(sys.argv[1])
end_ns = int(sys.argv[2])
print(f"{(end_ns - start_ns) / 1_000_000.0:.3f}")
PY
)
  status_text="ok"
  if [[ "$exit_code" -ne 0 ]]; then
    status_text="error"
  fi

  _tonic_obs_maybe_synthesize_child_run \
    "$child_run_id" \
    "$step_name" \
    "$started_at" \
    "$elapsed_ms" \
    "$exit_code" \
    "${cmd[@]}"

  _tonic_obs_safe_python finish-step \
    --output-root "$TONIC_OBS_DIR" \
    --run-id "$TONIC_OBS_RUN_ID" \
    --step "$step_name" \
    --child-run-id "$child_run_id" \
    --status "$status_text" \
    --exit-code "$exit_code" \
    --elapsed-ms "$elapsed_ms" \
    --at "$(_tonic_obs_now)" >/dev/null || true

  return "$exit_code"
}
