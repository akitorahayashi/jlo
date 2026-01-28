# Automated Pull Request Merging Strategy

## Strategy Outline
Auto-merge branches with specific prefixes (e.g., `jules/`) upon passing required status checks using GitHub Auto-merge.

## Prerequisites
1. **Repository Settings**: Enable "Allow auto-merge" in repository settings.
2. **Branch Protection**: Configure "Require status checks to pass before merging" for the target branch (e.g., `main`).

## Implementation Patterns

### Pattern A: Auto-Enable on PR Creation (Recommended)
Automatically enable auto-merge when a PR is opened.

```yaml
name: jules-squash-automerge
on:
  pull_request:
    types: [opened, reopened, ready_for_review]
permissions:
  pull-requests: write
  contents: write
jobs:
  enable:
    if: startsWith(github.head_ref, 'jules/') && github.event.pull_request.draft == false
    runs-on: ubuntu-latest
    steps:
      - name: Enable auto-merge (squash) + delete branch
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          pr="${{ github.event.pull_request.number }}"
          gh pr merge "$pr" --auto --squash --delete-branch --repo "${{ github.repository }}"
```

### Pattern B: Merge on Workflow Completion
Trigger merge explicitly after a specific workflow succeeds.

```yaml
name: jules-squash-merge-on-ci-success
on:
  workflow_run:
    workflows: ["CI"]
    types: [completed]
permissions:
  pull-requests: write
  contents: write
jobs:
  merge:
    if: github.event.workflow_run.conclusion == 'success'
    runs-on: ubuntu-latest
    steps:
      - name: Find PR for this branch and squash-merge
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          repo="${{ github.repository }}"
          branch="${{ github.event.workflow_run.head_branch }}"
          case "$branch" in
            jules/*) ;;
            *) echo "skip: $branch"; exit 0 ;;
          esac
          pr_number="$(gh pr list --repo "$repo" --head "$branch" --json number --jq '.[0].number')"
          if [ -z "$pr_number" ] || [ "$pr_number" = "null" ]; then
            exit 0
          fi
          gh pr merge "$pr_number" --squash --delete-branch --repo "$repo"
```

## Authentication & Permissions

### GitHub Actions Environment
`gh` CLI requires `GH_TOKEN` or `GITHUB_TOKEN`.

**Requirements:**
- **Permissions**: `pull-requests: write`, `contents: write`.
- **Environment Variable**: `GH_TOKEN: ${{ github.token }}`.

### Self-Hosted Runners
* **Running as GitHub Action**: Same configuration as GitHub-hosted runners (uses `GITHUB_TOKEN`).
* **Running as Independent Script**: Requires `GH_TOKEN` populated with a PAT (Personal Access Token) or `gh auth login` on the machine.
