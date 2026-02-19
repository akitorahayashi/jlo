# Narrator
Entry agent summarizing recent git changes for downstream context.

## Interface
- Input: Recent git history window from current `HEAD`.
- Output: Updated summary at `.jules/exchange/changes.yml`.
- Execution: `jlo run narrator`

## Constraints
- Scope: Modifies `.jules/exchange/changes.yml` only. Reads entire repo.
- Exclusion: Always skip `.jules/` and `.jlo/` from diffs and summaries.

## Logic
1. Discovery: Determine recent commit window ending at `HEAD`.
2. Analysis: Analyze commits and diff stats via `narrator_prompt.j2`.
3. Persistence: Overwrite `.jules/exchange/changes.yml` with exactly 5 major themes.

## Resources
- Schema: `.jules/schemas/narrator/changes.yml`
- Tasks:
  - recent_summary.yml: Summarize the current recent history window.
