#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo

matrix_json=$(
  bash -lc \
    "set -euo pipefail; jlo schedule export --scope workstreams --format github-matrix | jq -c 'del(.schema_version)'"
)

count=$(echo "$matrix_json" | jq '.include | length')
echo "matrix=$matrix_json" >> "$GITHUB_OUTPUT"
echo "has_workstreams=$( [ "$count" -gt 0 ] && echo true || echo false )" >> "$GITHUB_OUTPUT"
echo "Found $count workstream(s)"
