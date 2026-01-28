# Review: Jules Workflows

## Completed Changes

### 1. sync-jules.yml → workflow_call
- Changed from scheduled cron to `workflow_call`
- Called by `jules-workflows.yml` before observers
- Flow: `sync → observers` with explicit `needs` dependency

### 2. jules-workflows.yml Updates
- Added `sync` job before observers
- Changed push trigger from `main` to `jules` branch
- Added flow documentation in header

### 3. Observer/Decider/Planner Branch Configuration
- All three now use:
  - `checkout: ref: jules`
  - `starting_branch: jules`
- Implementer remains: `ref: main`, `starting_branch: main`

### 4. Auto-merge Double-Check
- Branch pattern: `jules/{observer,decider,planner}-*` only
- File scope: All changes must be within `.jules/`
- If either fails → no auto-merge (human review required)

---

## Verification Needed

| Item | Status |
|------|--------|
| `jules` branch exists in repository | ⬜ |
| Branch protection on `jules` allows workflow pushes | ⬜ |
| JULES_API_KEY secret configured | ⬜ |
| Repository setting "Allow auto-merge" enabled | ⬜ |

---

## Flow Summary

```
Daily 00:00 UTC:
  sync (main → jules) → observers (matrix) → [PRs to jules]

Daily 00:30 UTC:
  decider → [PR to jules]

On push to jules (issues/*.yml):
  planners (matrix) → [PRs to jules]

On push to jules (tasks/*.yml):
  implementers (matrix) → [PRs to main, branch: impl/*]

Auto-merge:
  jules/{observer,decider,planner}-* + .jules/ only → squash merge
  impl/* → human review required
```
