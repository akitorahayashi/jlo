#!/usr/bin/env bash
set -euo pipefail

: "${MOCK_TAG:?MOCK_TAG must be set}"
: "${REPO:?REPO must be set}"

require_command() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "::error::Required command not found: $name"
    exit 1
  fi
}

require_command gh
require_command git
require_command jq

if [ -z "${GH_TOKEN:-}" ]; then
  echo "::error::Missing GH_TOKEN; refusing to fall back to github.token."
  exit 1
fi

pr_numbers_json="${MOCK_PR_NUMBERS_JSON:-[]}"
branches_json="${MOCK_BRANCHES_JSON:-[]}"

echo "Closing mock PRs..."
for pr_number in $(echo "$pr_numbers_json" | jq -r '.[]'); do
  if [ -z "$pr_number" ]; then
    continue
  fi
  state=$(gh pr view "$pr_number" --repo "$REPO" --json state -q '.state' 2>/dev/null || true)
  if [ -z "$state" ]; then
    echo "::notice::PR #$pr_number not found; skipping"
    continue
  fi
  if [ "$state" = "OPEN" ]; then
    gh pr close "$pr_number" --repo "$REPO" || echo "::notice::Failed to close PR #$pr_number"
  fi
done

echo "Deleting mock branches..."
branches_from_outputs=$(echo "$branches_json" | jq -r '.[]')
branches_with_tag=$(gh api --paginate "/repos/$REPO/branches?per_page=100" | jq -r --arg tag "$MOCK_TAG" '.[] | select(.name | contains($tag)) | .name')
all_branches=$(printf '%s\n' "$branches_from_outputs" "$branches_with_tag" | sed '/^$/d' | sort -u)

for branch in $all_branches; do
  gh api -X DELETE "repos/$REPO/git/refs/heads/$branch" >/dev/null 2>&1 || \
    echo "::notice::Branch $branch not found or already deleted"
done

echo "Cleaning mock files from jules branch..."
git fetch origin jules
git checkout jules
git pull --ff-only origin jules

mapfile -t mock_files < <(find .jules/workstreams -type f \( -path "*/exchange/events/*" -o -path "*/exchange/issues/*" \) -name "*${MOCK_TAG}*")

if [ ${#mock_files[@]} -gt 0 ]; then
  printf '%s\n' "${mock_files[@]}" | xargs git rm -f
  git commit -m "Remove mock artifacts: ${MOCK_TAG}"
  git push origin jules
else
  echo "No mock files found for tag $MOCK_TAG"
fi

remaining=$(find .jules/workstreams -type f \( -path "*/exchange/events/*" -o -path "*/exchange/issues/*" \) -name "*${MOCK_TAG}*")
if [ -n "$remaining" ]; then
  echo "::error::Mock artifacts remain after cleanup:"
  echo "$remaining"
  exit 1
fi

echo "✅ Mock cleanup complete."
