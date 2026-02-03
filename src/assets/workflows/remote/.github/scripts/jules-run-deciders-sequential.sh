#!/usr/bin/env bash
set -euo pipefail

: "${DECIDER_MATRIX:?DECIDER_MATRIX must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo
require_command timeout

count=$(echo "$DECIDER_MATRIX" | jq '.include | length')
if [ "$count" -eq 0 ]; then
  echo "No deciders to run."
  exit 0
fi

echo "Running $count decider workstream(s) sequentially"
mapfile -t rows < <(echo "$DECIDER_MATRIX" | jq -c '.include[]')
for row in "${rows[@]}"; do
  workstream=$(echo "$row" | jq -r '.workstream')
  if [ -z "$workstream" ]; then
    echo "::error::Invalid decider matrix entry: $row"
    exit 1
  fi
  echo "Running decider for $workstream"
  timeout 20m jlo run deciders --workstream "$workstream" --scheduled
done
