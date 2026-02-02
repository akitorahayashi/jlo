#!/usr/bin/env bash
# Generate observer matrix at role level for GitHub Actions.
# Produces {"include": [{"workstream": "X", "role": "Y"}, ...]}
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

merged=$(
  echo "$WORKSTREAMS_JSON" | jq -r '.include[].workstream' | \
  xargs -I{} bash -lc 'set -euo pipefail; jlo schedule export --scope roles --layer observers --workstream "$1" --format github-matrix | jq -c "del(.schema_version)"' -- {} | \
  jq -sc 'map(.include) | add | {include: .}'
)
if [[ -z "$merged" || "$merged" == "null" ]]; then
  merged='{"include":[]}'
fi

count=$(echo "$merged" | jq '.include | length')
echo "matrix=$merged" >> "$GITHUB_OUTPUT"
if [ "$count" -gt 0 ]; then
  echo "has_observers=true" >> "$GITHUB_OUTPUT"
else
  echo "has_observers=false" >> "$GITHUB_OUTPUT"
fi
echo "Found $count observer role(s)"
