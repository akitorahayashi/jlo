[% if not use_matrix %]
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
require_command timeout

count=$(echo "$OBSERVER_MATRIX" | jq '.include | length')
if [ "$count" -eq 0 ]; then
  echo "No observer roles to run."
  exit 0
fi

echo "Running $count observer role(s) sequentially"
mapfile -t rows < <(echo "$OBSERVER_MATRIX" | jq -c '.include[]')
for row in "${rows[@]}"; do
  workstream=$(echo "$row" | jq -r '.workstream')
  role=$(echo "$row" | jq -r '.role')
  if [ -z "$workstream" ] || [ -z "$role" ]; then
    echo "::error::Invalid observer matrix entry: $row"
    exit 1
  fi
  echo "Running observer $workstream / $role"
  timeout 20m jlo run observers --workstream "$workstream" --role "$role"
done
[% endif %]
