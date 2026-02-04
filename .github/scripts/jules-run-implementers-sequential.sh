#!/usr/bin/env bash
set -euo pipefail

: "${IMPLEMENTER_MATRIX:?IMPLEMENTER_MATRIX must be set}"

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
mapfile -t issues < <(echo "$IMPLEMENTER_MATRIX" | jq -r '.include[]?.issue // empty')
if [ ${#issues[@]} -eq 0 ]; then
  echo "No implementers to run."
  exit 0
fi

echo "Running ${#issues[@]} implementer issue(s) sequentially"
for issue in "${issues[@]}"; do
  if [ -z "$issue" ]; then
    echo "::error::Empty issue path in matrix"
    exit 1
  fi
  if [ ! -f "$issue" ]; then
    echo "::error::Issue file not found in repository: $issue"
    exit 1
  fi
  echo "Running implementer for $issue"
  jlo run implementers "$issue" --branch "${TARGET_BRANCH:-main}"
  ISSUE_FILE="$issue" bash .github/scripts/jules-delete-processed-issue-and-events.sh
done
