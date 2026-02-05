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

# Extract issues directly using jq - single parse with null check
mapfile -t issues < <(echo "$PLANNER_MATRIX" | jq -r '.include[]?.issue // empty')
if [ ${#issues[@]} -eq 0 ]; then
  echo "No planners to run."
  exit 0
fi

echo "Running ${#issues[@]} planner issue(s) sequentially"
for issue in "${issues[@]}"; do
  if [ -z "$issue" ]; then
    echo "::error::Empty issue path in matrix"
    exit 1
  fi
  echo "Running planner for $issue"
  # shellcheck disable=SC2086
  jlo run planners "$issue" ${JLO_RUN_FLAGS:-}
done
