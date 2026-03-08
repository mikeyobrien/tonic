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
summary_file="$workspace/.ralph/agent/summary.md"
handoff_file="$workspace/.ralph/agent/handoff.md"

extract_excerpt() {
  label="$1"
  path="$2"
  lines="${3:-6}"

  [ -f "$path" ] || return 0

  printf '%s\n' "$label"
  python3 - "$path" "$lines" <<'PY'
import pathlib, re, sys
path = pathlib.Path(sys.argv[1])
limit = int(sys.argv[2])
text = path.read_text(errors="replace")
out = []
for raw in text.splitlines():
    line = raw.strip()
    if not line:
        continue
    line = re.sub(r'^#+\s*', '', line)
    line = re.sub(r'^[-*]\s+\[[ xX]\]\s*', '- ', line)
    line = re.sub(r'^[-*]\s+', '- ', line)
    line = line.replace('`', '')
    out.append(line)
    if len(out) >= limit:
        break
print("\n".join(out), end="")
PY
  printf '\n'
}

base_message=$(printf 'Loop complete\nrepo: %s\nloop: %s\niteration: %s\nbranch: %s\nhead: %s\nphase: %s' \
  "$repo_name" "$loop_id" "$iteration" "$branch" "$head" "$phase_event")

summary_block=""
if [ -f "$summary_file" ]; then
  summary_block=$(printf '\n\nsummary.md excerpt\npath: .ralph/agent/summary.md\n%s' "$(extract_excerpt "" "$summary_file" 6 | sed '/^$/d')")
fi

handoff_block=""
if [ -f "$handoff_file" ]; then
  handoff_block=$(printf '\n\nhandoff.md excerpt\npath: .ralph/agent/handoff.md\n%s' "$(extract_excerpt "" "$handoff_file" 8 | sed '/^$/d')")
fi

message="${base_message}${summary_block}${handoff_block}"
message="$(MESSAGE_INPUT="$message" python3 -c 'import os; text = os.environ.get("MESSAGE_INPUT", ""); max_len = 1000; print(text if len(text) <= max_len else text[:max_len - 1] + "…", end="")')"

title="Ralph complete: $repo_name"

curl -fsS https://api.pushover.net/1/messages.json \
  --form-string "token=$PUSHOVER_API_TOKEN" \
  --form-string "user=$PUSHOVER_USER_KEY" \
  --form-string "title=$title" \
  --form-string "message=$message" \
  >/dev/null

printf 'pushover hook: sent completion notification for %s (%s) with summary/handoff excerpts\n' "$repo_name" "$loop_id"
