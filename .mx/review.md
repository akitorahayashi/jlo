# Review: Jules Workflows

## Required Changes

### 1. Auto-merge Conditions (jules-automerge.yml)

**Current**: `startsWith(github.head_ref, 'jules/')`

**Required**: Double-check with branch pattern AND file scope.

```yaml
jobs:
  check-scope:
    if: |
      (startsWith(github.head_ref, 'jules/observer-') ||
       startsWith(github.head_ref, 'jules/decider-') ||
       startsWith(github.head_ref, 'jules/planner-')) &&
      github.event.pull_request.draft == false
    runs-on: ubuntu-latest
    outputs:
      jules_only: ${{ steps.check.outputs.jules_only }}
    steps:
      - name: Check changed files
        id: check
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          changed=$(gh pr view "${{ github.event.pull_request.number }}" \
            --repo "${{ github.repository }}" \
            --json files -q '.files[].path')
          non_jules=$(echo "$changed" | grep -v '^\.jules/' || true)
          if [ -z "$non_jules" ]; then
            echo "jules_only=true" >> "$GITHUB_OUTPUT"
          else
            echo "jules_only=false" >> "$GITHUB_OUTPUT"
            echo "::warning::PR modifies files outside .jules/: $non_jules"
          fi

  enable:
    needs: check-scope
    if: needs.check-scope.outputs.jules_only == 'true'
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
      contents: write
    steps:
      - name: Enable auto-merge
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh pr merge "${{ github.event.pull_request.number }}" \
            --auto --squash --delete-branch \
            --repo "${{ github.repository }}"
```

**Rationale**: Observer/Decider/Planner modify `.jules/` only. Any deviation must stop.

---

### 2. Starting Branch for Observer/Decider/Planner

**Current**: `starting_branch: main`

**Required**: `starting_branch: jules`

**Files to update**:
- `run-observer.yml` L34: `starting_branch: jules`
- `run-decider.yml` L42: `starting_branch: jules`
- `run-planner.yml` L52: `starting_branch: jules`

**Implementer remains**: `starting_branch: main` (correct)

---

### 3. sync-jules.yml Target Update

**Current**: Syncs `main → jules` daily before Observers.

**No change needed**: This ensures jules branch has latest main code.

---

## Summary

| Agent | Starting Branch | Auto-merge |
|-------|-----------------|------------|
| Observer | `jules` | ✅ if `.jules/` only |
| Decider | `jules` | ✅ if `.jules/` only |
| Planner | `jules` | ✅ if `.jules/` only |
| Implementer | `main` | ❌ (human review) |
