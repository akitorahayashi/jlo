#!/usr/bin/env bash
set -euo pipefail

: "${PLANNER_MATRIX:?PLANNER_MATRIX must be set}"

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

count=$(echo "$PLANNER_MATRIX" | jq '.include | length')
if [ "$count" -eq 0 ]; then
  echo "No planners to run."
  exit 0
fi

echo "Running $count planner issue(s) sequentially"
mapfile -t rows < <(echo "$PLANNER_MATRIX" | jq -c '.include[]')
for row in "${rows[@]}"; do
  issue=$(echo "$row" | jq -r '.issue')
  if [ -z "$issue" ]; then
    echo "::error::Invalid planner matrix entry: $row"
    exit 1
  fi
  echo "Running planner for $issue"
  timeout 20m jlo run planners "$issue"
done
