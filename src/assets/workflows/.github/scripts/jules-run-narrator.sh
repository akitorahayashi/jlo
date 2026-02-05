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

require_command jlo
require_command jq

run_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "run_started_at=$run_started_at" >> "$GITHUB_OUTPUT"

echo "Running narrator"
# shellcheck disable=SC2086
output=$(env -u GITHUB_OUTPUT jlo run narrator ${JLO_RUN_FLAGS:-} 2>&1 | tee /dev/stderr)

expected_count=1
if echo "$output" | grep -q "Skipping Narrator"; then
  expected_count=0
fi

mock_branches=()
mock_prs=()
mock_tag=""

while IFS= read -r line; do
  case "$line" in
    MOCK_BRANCH=*)
      mock_branches+=("${line#MOCK_BRANCH=}")
      ;;
    MOCK_PR_NUMBER=*)
      mock_prs+=("${line#MOCK_PR_NUMBER=}")
      ;;
    MOCK_TAG=*)
      value="${line#MOCK_TAG=}"
      if [ -z "$mock_tag" ]; then
        mock_tag="$value"
      elif [ "$mock_tag" != "$value" ]; then
        echo "::error::Mock tag mismatch: $mock_tag vs $value"
        exit 1
      fi
      ;;
  esac
done <<< "$output"

pr_numbers_json=$(printf '%s\n' "${mock_prs[@]}" | jq -R . | jq -s 'map(select(length>0))')
branches_json=$(printf '%s\n' "${mock_branches[@]}" | jq -R . | jq -s 'map(select(length>0))')

echo "expected_count=$expected_count" >> "$GITHUB_OUTPUT"
echo "mock_pr_numbers_json=$pr_numbers_json" >> "$GITHUB_OUTPUT"
echo "mock_branches_json=$branches_json" >> "$GITHUB_OUTPUT"
if [ -n "$mock_tag" ]; then
  echo "mock_tag=$mock_tag" >> "$GITHUB_OUTPUT"
fi
