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

# Extract workstreams directly using jq - single parse with null check
mapfile -t workstreams < <(echo "$DECIDER_MATRIX" | jq -r '.include[]?.workstream // empty')
if [ ${#workstreams[@]} -eq 0 ]; then
  echo "No deciders to run."
  exit 0
fi

echo "Running ${#workstreams[@]} decider workstream(s) sequentially"
for workstream in "${workstreams[@]}"; do
  if [ -z "$workstream" ]; then
    echo "::error::Empty workstream in matrix"
    exit 1
  fi
  echo "Running decider for $workstream"
  # shellcheck disable=SC2086
  jlo run deciders --workstream "$workstream" --scheduled ${JLO_RUN_FLAGS:-}
done
