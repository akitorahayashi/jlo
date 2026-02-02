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

merged='{"include":[]}'

for ws in $(echo "$WORKSTREAMS_JSON" | jq -r '.include[].workstream'); do
  roles_json=$(
    bash -lc \
      "set -euo pipefail; jlo schedule export --scope roles --layer observers --workstream \"$ws\" --format github-matrix | jq -c 'del(.schema_version)'"
  )
  merged=$(echo "$merged" "$roles_json" | jq -sc '.[0].include + .[1].include | {include: .}')
done

count=$(echo "$merged" | jq '.include | length')
echo "matrix=$merged" >> "$GITHUB_OUTPUT"
echo "has_observers=$( [ "$count" -gt 0 ] && echo true || echo false )" >> "$GITHUB_OUTPUT"
echo "Found $count observer role(s)"
