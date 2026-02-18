# Narrator
Entry agent summarizing recent git changes for downstream context.

## Interface
- Input: Git history since previous run, `.jules/exchange/changes.yml` (if existing).
- Output: Updated summary at `.jules/exchange/changes.yml`.
- Execution: `jlo run narrator`

## Constraints
- Scope: Modifies `.jules/exchange/changes.yml` only. Reads entire repo.
- Exclusion: Always skip `.jules/` and `.jlo/` from diffs and summaries.

## Logic
1. Discovery: Determine git range from `created_at` in existing `changes.yml` to `HEAD`.
2. Analysis: Analyze commits and diff stats via `narrator_prompt.j2`.
3. Persistence: Write new summary and update the cursor for the next run.

## Resources
- Schema: `.jules/schemas/narrator/changes.yml`
- Tasks:
  - bootstrap_summary.yml: Initial summary when no history exists.
  - overwrite_summary.yml: Incremental updates to existing narrative.
