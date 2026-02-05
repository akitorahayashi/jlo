#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set}"
: "${WORKSTREAMS_JSON:?WORKSTREAMS_JSON must be set}"
: "${ROUTING_LABELS:?ROUTING_LABELS must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command jq
require_command jlo

workstreams=$(echo "$WORKSTREAMS_JSON" | jq -r '.include[].workstream')

labels_json=$(printf '%s' "$ROUTING_LABELS" | jq -R 'split(",") | map(gsub("^\\s+|\\s+$";"")) | map(select(length>0))')
if [ "$(echo "$labels_json" | jq 'length')" -eq 0 ]; then
  echo "::error::ROUTING_LABELS resolved to an empty list"
  exit 1
fi
echo "Routing labels: $ROUTING_LABELS"

planner_list=""
implementer_list=""

for ws in $workstreams; do
  inspect=$(jlo workstreams inspect --workstream "$ws" --format json)
  echo "Workstream $ws: $(echo "$inspect" | jq -c '{issues_count: (.issues.items | length)}')"

  planner_issues=$(echo "$inspect" | jq -r --argjson labels "$labels_json" '
    .issues.items[]
    | select(type == "object")
    | select(.label != null)
    | select(.label | IN($labels[]))
    | select(.requires_deep_analysis == true)
    | .path
  ')
  for issue in $planner_issues; do
    planner_list="${planner_list}${issue}"$'\n'
    echo "Planner candidate: $issue"
  done

  implementer_issues=$(echo "$inspect" | jq -r --argjson labels "$labels_json" '
    .issues.items[]
    | select(type == "object")
    | select(.label != null)
    | select(.label | IN($labels[]))
    | select(.requires_deep_analysis == false)
    | .path
  ')
  for issue in $implementer_issues; do
    implementer_list="${implementer_list}${issue}"$'\n'
    echo "Implementer candidate: $issue"
  done
done

planner_list=$(echo -n "$planner_list" | sed '/^$/d')
implementer_list=$(echo -n "$implementer_list" | sed '/^$/d')

if [ -z "$planner_list" ]; then
  echo "planner_matrix={\"include\":[]}" >> "$GITHUB_OUTPUT"
  echo "has_planners=false" >> "$GITHUB_OUTPUT"
else
  planner_matrix=$(echo "$planner_list" | jq -R . | jq -sc '{include: [.[] | {issue: .}]}')
  echo "planner_matrix=$planner_matrix" >> "$GITHUB_OUTPUT"
  echo "has_planners=true" >> "$GITHUB_OUTPUT"
fi

if [ -z "$implementer_list" ]; then
  echo "implementer_matrix={\"include\":[]}" >> "$GITHUB_OUTPUT"
  echo "has_implementers=false" >> "$GITHUB_OUTPUT"
else
  implementer_matrix=$(echo "$implementer_list" | jq -R . | jq -sc '{include: [.[] | {issue: .}]}')
  echo "implementer_matrix=$implementer_matrix" >> "$GITHUB_OUTPUT"
  echo "has_implementers=true" >> "$GITHUB_OUTPUT"
fi
