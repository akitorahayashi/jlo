#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"
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

run_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "run_started_at=$run_started_at" >> "$GITHUB_OUTPUT"

# Extract issues directly using jq - single parse with null check
mapfile -t issues < <(echo "$IMPLEMENTER_MATRIX" | jq -r '.include[]?.issue // empty')
if [ ${#issues[@]} -eq 0 ]; then
  echo "No implementers to run."
  echo "expected_count=0" >> "$GITHUB_OUTPUT"
  echo "mock_pr_numbers_json=[]" >> "$GITHUB_OUTPUT"
  echo "mock_branches_json=[]" >> "$GITHUB_OUTPUT"
  exit 0
fi

mock_branches=()
mock_prs=()
mock_tag=""

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
  # shellcheck disable=SC2086
  output=$(env -u GITHUB_OUTPUT jlo run implementers "$issue" --branch "${TARGET_BRANCH:-main}" ${JLO_RUN_FLAGS:-} 2>&1 | tee /dev/stderr)

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

  # Skip cleanup in mock mode
  if [ -z "${JLO_RUN_FLAGS:-}" ] || [[ ! "${JLO_RUN_FLAGS:-}" =~ --mock ]]; then
    ISSUE_FILE="$issue" bash .github/scripts/jules-delete-processed-issue-and-events.sh
  fi
done

pr_numbers_json=$(printf '%s\n' "${mock_prs[@]}" | jq -R . | jq -s 'map(select(length>0))')
branches_json=$(printf '%s\n' "${mock_branches[@]}" | jq -R . | jq -s 'map(select(length>0))')

echo "expected_count=${#issues[@]}" >> "$GITHUB_OUTPUT"
echo "mock_pr_numbers_json=$pr_numbers_json" >> "$GITHUB_OUTPUT"
echo "mock_branches_json=$branches_json" >> "$GITHUB_OUTPUT"
if [ -n "$mock_tag" ]; then
  echo "mock_tag=$mock_tag" >> "$GITHUB_OUTPUT"
fi
