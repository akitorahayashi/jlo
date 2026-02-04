#!/usr/bin/env bash
set -euo pipefail

: "${IMPLEMENTER_MATRIX:?IMPLEMENTER_MATRIX must be set}"
: "${TARGET_BRANCH:?TARGET_BRANCH must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo

count=$(echo "$IMPLEMENTER_MATRIX" | jq '.include | length')
if [ "$count" -eq 0 ]; then
  echo "No implementers to run."
  exit 0
fi

echo "Running $count implementer issue(s) sequentially"
mapfile -t rows < <(echo "$IMPLEMENTER_MATRIX" | jq -c '.include[]')
for row in "${rows[@]}"; do
  issue=$(echo "$row" | jq -r '.issue')
  if [ -z "$issue" ]; then
    echo "::error::Invalid implementer matrix entry: $row"
    exit 1
  fi
  if [ ! -f "$issue" ]; then
    echo "::error::Issue file not found in repository: $issue"
    exit 1
  fi
  echo "Running implementer for $issue"
  jlo run implementers "$issue" --branch "$TARGET_BRANCH"
  ISSUE_FILE="$issue" bash .github/scripts/jules-delete-processed-issue-and-events.sh
done
