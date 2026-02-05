#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"
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

run_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "run_started_at=$run_started_at" >> "$GITHUB_OUTPUT"

# Extract workstreams directly using jq - single parse with null check
mapfile -t workstreams < <(echo "$DECIDER_MATRIX" | jq -r '.include[]?.workstream // empty')
if [ ${#workstreams[@]} -eq 0 ]; then
  echo "No deciders to run."
  echo "expected_count=0" >> "$GITHUB_OUTPUT"
  echo "mock_pr_numbers_json=[]" >> "$GITHUB_OUTPUT"
  echo "mock_branches_json=[]" >> "$GITHUB_OUTPUT"
  exit 0
fi

mock_branches=()
mock_prs=()
mock_scope=""

echo "Running ${#workstreams[@]} decider workstream(s) sequentially"
for workstream in "${workstreams[@]}"; do
  if [ -z "$workstream" ]; then
    echo "::error::Empty workstream in matrix"
    exit 1
  fi
  echo "Running decider for $workstream"
  # shellcheck disable=SC2086
  output=$(env -u GITHUB_OUTPUT jlo run deciders --workstream "$workstream" --scheduled ${JLO_RUN_FLAGS:-} 2>&1 | tee /dev/stderr)

  while IFS= read -r line; do
    case "$line" in
      MOCK_BRANCH=*)
        mock_branches+=("${line#MOCK_BRANCH=}")
        ;;
      MOCK_PR_NUMBER=*)
        mock_prs+=("${line#MOCK_PR_NUMBER=}")
        ;;
      MOCK_SCOPE=*)
        value="${line#MOCK_SCOPE=}"
        if [ -z "$mock_scope" ]; then
          mock_scope="$value"
        elif [ "$mock_scope" != "$value" ]; then
          echo "::error::Mock scope mismatch: $mock_scope vs $value"
          exit 1
        fi
        ;;
    esac
  done <<< "$output"
done

pr_numbers_json=$(printf '%s\n' "${mock_prs[@]}" | jq -R . | jq -s 'map(select(length>0))')
branches_json=$(printf '%s\n' "${mock_branches[@]}" | jq -R . | jq -s 'map(select(length>0))')

echo "expected_count=${#workstreams[@]}" >> "$GITHUB_OUTPUT"
echo "mock_pr_numbers_json=$pr_numbers_json" >> "$GITHUB_OUTPUT"
echo "mock_branches_json=$branches_json" >> "$GITHUB_OUTPUT"
if [ -n "$mock_scope" ]; then
  echo "mock_scope=$mock_scope" >> "$GITHUB_OUTPUT"
fi
