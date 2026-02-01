#!/usr/bin/env bash
set -euo pipefail

: "${ISSUE_FILE:?ISSUE_FILE must be set}"

COMMIT_CHANGES="${COMMIT_CHANGES:-true}"
PUSH_CHANGES="${PUSH_CHANGES:-true}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command awk
require_command git
require_command jq
require_command jlo

if [ ! -f "$ISSUE_FILE" ]; then
  echo "::error::Issue file not found: $ISSUE_FILE"
  exit 1
fi

WORKSTREAM=$(echo "$ISSUE_FILE" | awk -F'/workstreams/' '{print $2}' | awk -F'/' '{print $1}')
if [ -z "$WORKSTREAM" ]; then
  echo "::error::Could not determine workstream from $ISSUE_FILE"
  exit 1
fi

inspect=$(jlo workstreams inspect --workstream "$WORKSTREAM" --format json)

SOURCE_EVENTS=$(echo "$inspect" | jq -c --arg path "$ISSUE_FILE" '.issues.items[] | select(.path == $path) | .source_events')
if [ -z "$SOURCE_EVENTS" ] || [ "$SOURCE_EVENTS" = "null" ]; then
  echo "::error::source_events not found for $ISSUE_FILE"
  exit 1
fi

EVENT_PATHS=$(echo "$inspect" | jq -r --argjson ids "$SOURCE_EVENTS" '
  [ .events.items[] | select(.id as $id | ($ids | index($id)) != null) | .path ] | .[]
')

if [ -z "$EVENT_PATHS" ]; then
  echo "::error::No event paths found for source_events in $ISSUE_FILE"
  exit 1
fi

echo "Deleting source events:"
echo "$EVENT_PATHS"
while read -r path; do
  if [ -z "$path" ]; then
    continue
  fi
  if [ ! -f "$path" ]; then
    echo "::error::Source event file not found: $path"
    exit 1
  fi
  git rm "$path"
done <<< "$EVENT_PATHS"

git rm "$ISSUE_FILE"
if [ "$COMMIT_CHANGES" = "true" ]; then
  git commit -m "Remove processed issue and source events: $(basename "$ISSUE_FILE")"
else
  echo "Skipping commit (COMMIT_CHANGES=$COMMIT_CHANGES)"
fi

if [ "$PUSH_CHANGES" = "true" ]; then
  git push origin jules
else
  echo "Skipping push (PUSH_CHANGES=$PUSH_CHANGES)"
fi

echo "✅ Cleanup complete."
