#!/usr/bin/env sh
set -eu

CONFIG_FILE="${RALPH_PUSHOVER_CONFIG:-$HOME/.config/ralph/pushover.env}"

if [ ! -f "$CONFIG_FILE" ]; then
  echo "pushover hook: missing config file: $CONFIG_FILE" >&2
  exit 1
fi

# shellcheck disable=SC1090
. "$CONFIG_FILE"

: "${PUSHOVER_USER_KEY:?pushover hook: missing PUSHOVER_USER_KEY}"
: "${PUSHOVER_API_TOKEN:?pushover hook: missing PUSHOVER_API_TOKEN}"

phase_event="${RALPH_HOOK_PHASE_EVENT:-post.loop.complete}"
loop_id="${RALPH_LOOP_ID:-unknown-loop}"
workspace="${RALPH_WORKSPACE:-$PWD}"
iteration="${RALPH_ITERATION:-unknown}"
repo_name="$(basename "$workspace")"
branch="$(git -C "$workspace" rev-parse --abbrev-ref HEAD 2>/dev/null || printf 'unknown-branch')"
head="$(git -C "$workspace" rev-parse --short HEAD 2>/dev/null || printf 'unknown-head')"
summary_file="$workspace/.ralph/agent/handoff.md"

summary_line=""
if [ -f "$summary_file" ]; then
  summary_line="handoff: .ralph/agent/handoff.md"
fi

message=$(printf 'Loop complete\nrepo: %s\nloop: %s\niteration: %s\nbranch: %s\nhead: %s\nphase: %s%s' \
  "$repo_name" "$loop_id" "$iteration" "$branch" "$head" "$phase_event" \
  "$( [ -n "$summary_line" ] && printf '\n%s' "$summary_line" || true )")

title="Ralph complete: $repo_name"

curl -fsS https://api.pushover.net/1/messages.json \
  --form-string "token=$PUSHOVER_API_TOKEN" \
  --form-string "user=$PUSHOVER_USER_KEY" \
  --form-string "title=$title" \
  --form-string "message=$message" \
  >/dev/null

printf 'pushover hook: sent completion notification for %s (%s)\n' "$repo_name" "$loop_id"
