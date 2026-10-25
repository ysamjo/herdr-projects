#!/bin/sh
# Bridge CodeBuddy Code lifecycle events into Herdr's agent state API.
#
# Usage: herdr-agent.sh start|working|idle|blocked|release
# CodeBuddy hook JSON is read from stdin.
#
# Does nothing unless CodeBuddy is running inside a Herdr-managed pane.

set -u

action=${1:-}

[ "${HERDR_ENV:-}" = "1" ] || exit 0
[ -n "${HERDR_PANE_ID:-}" ] || exit 0

herdr=${HERDR_BIN_PATH:-herdr}
if [ -x "$herdr" ]; then
  :
elif command -v "$herdr" >/dev/null 2>&1; then
  :
else
  exit 0
fi

input=$(cat 2>/dev/null | tr '\n' ' ' || true)

field() {
  printf '%s' "$input" |
    sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" |
    head -n 1
}

session_id=$(field session_id)
permission_mode=$(field permission_mode)

source_id=codebuddy-plugin
agent_name=codebuddy

state_dir=${TMPDIR:-/tmp}/codebuddy-herdr
mkdir -p "$state_dir" 2>/dev/null || exit 0
key=$(printf '%s' "$HERDR_PANE_ID" | tr -c 'A-Za-z0-9._-' '_')
state_file=$state_dir/$key.state
seq_file=$state_dir/$key.seq

next_seq() {
  now=$(date +%s)
  last=$(cat "$seq_file" 2>/dev/null || echo 0)
  case $last in
    '' | *[!0-9]*) last=0 ;;
  esac
  seq=$((now * 1000))
  [ "$seq" -le "$last" ] && seq=$((last + 1))
  printf '%s\n' "$seq" >"$seq_file" 2>/dev/null || true
  printf '%s' "$seq"
}

report() {
  state=$1
  message=${2:-}

  set -- "$herdr" pane report-agent "$HERDR_PANE_ID" \
    --source "$source_id" --agent "$agent_name" --state "$state" --seq "$(next_seq)"

  [ -n "$message" ] && set -- "$@" --message "$message"
  [ -n "$session_id" ] && set -- "$@" --agent-session-id "$session_id"

  if [ -n "$session_id" ]; then
    set -- "$@" -- codebuddy --resume "$session_id"
    case $permission_mode in
      default | plan | acceptEdits | bypassPermissions | dontAsk | auto)
        set -- "$@" --permission-mode "$permission_mode"
        ;;
    esac
  fi

  if "$@" >/dev/null 2>&1; then
    printf '%s\n' "$state" >"$state_file" 2>/dev/null || true
  fi
}

last_state=$(cat "$state_file" 2>/dev/null || true)

case $action in
  start | resume)
    report idle
    ;;
  working)
    [ "$last_state" = "working" ] || report working
    ;;
  idle)
    [ "$last_state" = "idle" ] || report idle
    ;;
  blocked)
    message=$(field message)
    [ -n "$message" ] || message="Waiting for your input"
    [ "$last_state" = "blocked" ] || report blocked "$message"
    ;;
  release)
    "$herdr" pane release-agent "$HERDR_PANE_ID" \
      --source "$source_id" --agent "$agent_name" --seq "$(next_seq)" >/dev/null 2>&1 || true
    rm -f "$state_file"
    ;;
esac

exit 0
