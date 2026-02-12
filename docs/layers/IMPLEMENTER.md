# Implementer Role Guide

The **Implementer** is the final execution agent in the loop, responsible for writing code to satisfy requirements.

## Purpose

The Implementer takes structured requirements (and plans from the Planner) and translates them into code changes, tests, and documentation. It ensures that the changes are correct, tested, and follow project standards.

## Inputs

- **Requirements**: YAML files in `.jules/exchange/requirements/`.
- **Plans**: Detailed execution steps provided by the Planner (within the requirement file).
- **Codebase Context**: Full access to modify the codebase.

## Outputs

- **Pull Requests**: Creates a new branch and PR for each requirement implemented.
- **Code Changes**: Modifies source files, adds tests, updates docs.

## Execution

The Implementer is executed via the CLI, pointing to a specific requirement:

```bash
jlo run implementer .jules/exchange/requirements/<requirement-id>.yml
```

### Process Flow

1. **Context Loading**:
   - Reads the requirement and any attached plan.
   - Loads relevant files mentioned in `affected_areas` or `files_to_touch`.

2. **Implementation**:
   - Iteratively modifies code to meet the acceptance criteria.
   - Writes tests to verify the changes.
   - Updates documentation if needed.

3. **Verification**:
   - Runs tests (if possible/configured) to ensure correctness.
   - Checks for linting errors.

4. **Submission**:
   - Creates a new branch (`jules-implementer-<label>-<id>`).
   - Commits changes.
   - Pushes the branch and creates a Pull Request.

## Tasks

The Implementer's prompt (`implementer_prompt.j2`) receives the task description directly from the requirement file it is executing. This guides the agent through the implementation steps.

## Mock Mode

Running with `--mock` allows testing the workflow without modifying code:

```bash
jlo run implementer .jules/exchange/requirements/example.yml --mock
```

This creates a dummy PR with the correct branch name but no actual code changes.

## Troubleshooting

- **Merge Conflicts**: If the base branch has moved significantly, the Implementer might encounter conflicts. Rebase the PR branch onto `JLO_TARGET_BRANCH` (e.g. `main`).
- **Failed Tests**: The Implementer attempts to fix broken tests, but complex failures may require human intervention.
