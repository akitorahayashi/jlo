#!/usr/bin/env bash
set -euo pipefail

: "${OBSERVER_MATRIX:?OBSERVER_MATRIX must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo

# Extract workstream/role pairs using a single jq call (tab-separated)
mapfile -t entries < <(echo "$OBSERVER_MATRIX" | jq -r '.include[]? | "\(.workstream)\t\(.role)"')
if [ ${#entries[@]} -eq 0 ]; then
  echo "No observer roles to run."
  exit 0
fi

echo "Running ${#entries[@]} observer role(s) sequentially"
for entry in "${entries[@]}"; do
  IFS=$'\t' read -r workstream role <<< "$entry"
  if [ -z "$workstream" ] || [ -z "$role" ]; then
    echo "::error::Invalid observer matrix entry: missing workstream or role"
    exit 1
  fi
  echo "Running observer $workstream / $role"
  # shellcheck disable=SC2086
  jlo run observers --workstream "$workstream" --role "$role" ${JLO_RUN_FLAGS:-}
done
