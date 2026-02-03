#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"
: "${WORKSTREAMS_JSON:?WORKSTREAMS_JSON must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo

workstreams=$(echo "$WORKSTREAMS_JSON" | jq -r '.include[].workstream')

pending_list=""
for ws in $workstreams; do
  inspect=$(jlo workstreams inspect --workstream "$ws" --format json)
  pending=$(echo "$inspect" | jq '[.events.items[] | select(.state == "pending")] | length')
  if [ "$pending" -gt 0 ]; then
    pending_list="${pending_list}${ws}"$'\n'
    echo "Workstream $ws has $pending pending event(s)"
  else
    echo "Workstream $ws has no pending events"
  fi
done

pending_list=$(echo -n "$pending_list" | sed '/^$/d')

if [ -z "$pending_list" ]; then
  echo "matrix={\"include\":[]}" >> "$GITHUB_OUTPUT"
  echo "has_pending=false" >> "$GITHUB_OUTPUT"
else
  matrix_json=$(echo "$pending_list" | jq -R . | jq -sc '{include: [.[] | {workstream: .}]}')
  echo "matrix=$matrix_json" >> "$GITHUB_OUTPUT"
  echo "has_pending=true" >> "$GITHUB_OUTPUT"
fi
