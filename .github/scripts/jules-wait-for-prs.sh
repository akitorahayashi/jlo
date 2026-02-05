#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"
: "${EXPECTED_COUNT:?EXPECTED_COUNT must be set}"
: "${WAIT_MODE:?WAIT_MODE must be set (merge|label)}"
: "${RUN_STARTED_AT:?RUN_STARTED_AT must be set}"
: "${BASE_BRANCH:?BASE_BRANCH must be set}"
: "${CONTRACTS_PATH:?CONTRACTS_PATH must be set}"
: "${REPO:?REPO must be set}"
: "${WAIT_MINUTES:?WAIT_MINUTES must be set}"
: "${MOCK_MODE:?MOCK_MODE must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command awk
require_command gh
require_command jq

if [ "$EXPECTED_COUNT" -eq 0 ]; then
  echo "No PRs expected. Skipping wait."
  echo "pr_numbers_json=[]" >> "$GITHUB_OUTPUT"
  echo "pr_heads_json=[]" >> "$GITHUB_OUTPUT"
  exit 0
fi

if [ ! -f "$CONTRACTS_PATH" ]; then
  echo "::error::Missing contracts file: $CONTRACTS_PATH"
  exit 1
fi

branch_prefix=$(awk -F':' '/^branch_prefix:/ {gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); gsub(/["'\'']/, "", $2); print $2; exit}' "$CONTRACTS_PATH")
if [ -z "$branch_prefix" ]; then
  echo "::error::Missing branch_prefix in $CONTRACTS_PATH"
  exit 1
fi

timeout_seconds=$((WAIT_MINUTES * 60))
if [ "$MOCK_MODE" = "true" ]; then
  timeout_seconds=30
fi

poll_interval=30
deadline=$(( $(date +%s) + timeout_seconds ))

labels_path=".jules/github-labels.json"
if [ "$WAIT_MODE" = "label" ] && [ ! -f "$labels_path" ]; then
  echo "::error::Missing label definition file: $labels_path"
  exit 1
fi

find_pr_numbers() {
  local pr_numbers_json
  if [ -n "${MOCK_PR_NUMBERS_JSON:-}" ] && [ "$MOCK_PR_NUMBERS_JSON" != "null" ] && [ "$MOCK_PR_NUMBERS_JSON" != "[]" ]; then
    pr_numbers_json="$MOCK_PR_NUMBERS_JSON"
  else
    pr_numbers_json=$(gh api --paginate "/repos/$REPO/pulls?state=all&per_page=100" | jq -c --arg prefix "$branch_prefix" --arg base "$BASE_BRANCH" --arg start "$RUN_STARTED_AT" '
      [ .[]
        | select(.base.ref == $base)
        | select(.head.ref | startswith($prefix))
        | select(.created_at >= $start)
        | .number
      ]
      | unique
      | sort
    ')
  fi
  echo "$pr_numbers_json"
}

while [ "$(date +%s)" -le "$deadline" ]; do
  pr_numbers_json=$(find_pr_numbers)
  pr_count=$(echo "$pr_numbers_json" | jq 'length')

  if [ "$pr_count" -lt "$EXPECTED_COUNT" ]; then
    echo "Waiting for PRs: $pr_count/$EXPECTED_COUNT discovered."
    sleep "$poll_interval"
    continue
  fi

  pr_heads=()
  all_ready=true

  for pr_number in $(echo "$pr_numbers_json" | jq -r '.[]'); do
    pr_json=$(gh pr view "$pr_number" --repo "$REPO" --json state,mergedAt,headRefName,labels)
    state=$(echo "$pr_json" | jq -r '.state')
    merged_at=$(echo "$pr_json" | jq -r '.mergedAt')
    head_ref=$(echo "$pr_json" | jq -r '.headRefName')
    pr_heads+=("$head_ref")

    if [ "$WAIT_MODE" = "merge" ]; then
      if [ "$state" = "CLOSED" ] && [ "$merged_at" = "null" ]; then
        echo "::error::PR #$pr_number was closed without merge."
        exit 1
      fi
      if [ "$merged_at" = "null" ] || [ -z "$merged_at" ]; then
        all_ready=false
      fi
    elif [ "$WAIT_MODE" = "label" ]; then
      if [ "$state" != "OPEN" ]; then
        echo "::error::PR #$pr_number is not open (state: $state)."
        exit 1
      fi

      if [[ "$head_ref" != "$branch_prefix"* ]]; then
        echo "::error::PR #$pr_number head ref does not match prefix '$branch_prefix': $head_ref"
        exit 1
      fi

      suffix="${head_ref#"$branch_prefix"}"
      label="${suffix%%-*}"
      remainder="${suffix#"$label-"}"
      issue_id="${remainder%%-*}"

      if [ -z "$label" ] || [ -z "$issue_id" ]; then
        echo "::error::Could not parse label/id from branch '$head_ref'"
        exit 1
      fi
      if [ "${#issue_id}" -ne 6 ] || [[ ! "$issue_id" =~ ^[a-z0-9]{6}$ ]]; then
        echo "::error::Invalid issue id '$issue_id' in branch '$head_ref'"
        exit 1
      fi

      if ! jq -e --arg label "$label" '.issue_labels[$label] // empty' "$labels_path" >/dev/null; then
        echo "::error::Label '$label' is not defined in $labels_path"
        exit 1
      fi

      labels=$(echo "$pr_json" | jq -r '.labels[].name')
      if ! echo "$labels" | grep -Fxq "$label"; then
        all_ready=false
      fi
    else
      echo "::error::Unknown WAIT_MODE '$WAIT_MODE'"
      exit 1
    fi
  done

  if [ "$all_ready" = "true" ]; then
    pr_heads_json=$(printf '%s\n' "${pr_heads[@]}" | jq -R . | jq -s 'map(select(length>0))')
    echo "pr_numbers_json=$pr_numbers_json" >> "$GITHUB_OUTPUT"
    echo "pr_heads_json=$pr_heads_json" >> "$GITHUB_OUTPUT"
    echo "Ready: $WAIT_MODE conditions satisfied for $pr_count PR(s)."
    exit 0
  fi

  echo "Waiting for $WAIT_MODE conditions to be satisfied..."
  sleep "$poll_interval"
done

echo "::error::Timed out waiting for $WAIT_MODE conditions."
exit 1
