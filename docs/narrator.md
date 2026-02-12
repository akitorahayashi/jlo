# Narrator Role Guide

The **Narrator** is the first agent in the Jules workflow loop. Its primary responsibility is to create a summarized narrative of recent changes in the codebase, which serves as context for subsequent agents (Observers, Decider, etc.).

## Purpose

The Narrator reads the git history since the last run and produces a high-level summary of changes. This summary helps other agents understand *what* happened recently without needing to analyze every commit in detail themselves.

## Inputs

- **Git History**: Commits since the last cursor (stored in `.jules/exchange/changes.yml`).
- **Previous Summary**: The existing `.jules/exchange/changes.yml` file (if any).

## Outputs

- **Changes Summary**: A YAML file located at `.jules/exchange/changes.yml`.

### Schema (`changes.yml`)

The output adheres to the schema defined in `.jules/roles/narrator/schemas/changes.yml`. It typically includes:

- `created_at`: Timestamp of the run.
- `from_commit`: Starting commit hash of the range.
- `to_commit`: Ending commit hash of the range.
- `summary`: A textual summary of the changes.
- `impact_analysis`: High-level analysis of potential impacts.

## Execution

The Narrator is executed via the CLI:

```bash
jlo run narrator
```

### Process Flow

1. **Range Determination**:
   - Reads `.jules/exchange/changes.yml` to find the `created_at` timestamp of the last run.
   - Identifies commits from that timestamp up to `HEAD`.
   - If no previous summary exists, it bootstraps by analyzing the last N commits.

2. **Analysis**:
   - Sends the commit logs and diff stats to the LLM.
   - Requests a summary based on the `narrator_prompt.j2` template.

3. **Output Generation**:
   - Writes the new `changes.yml` to the exchange.
   - Updates the cursor for the next run.

## Tasks

The Narrator performs specific tasks defined in `.jules/roles/narrator/tasks/`. The main prompt, `narrator_prompt.j2`, dynamically includes a task based on the run mode:

- `bootstrap_summary`: Used when no previous history exists.
- `overwrite_summary`: Used to update the existing summary with new changes (incremental).

## Troubleshooting

- **No Changes Detected**: If no changes occurred outside `.jules/` or `.jlo/`, the Narrator may skip execution.
- **Cursor Drift**: If the git history is rewritten (rebase), the Narrator might lose its place. In this case, deleting `.jules/exchange/changes.yml` forces a bootstrap run.
