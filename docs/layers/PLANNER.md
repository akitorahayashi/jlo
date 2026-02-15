# Planner Role Guide

The **Planner** agent performs deep analysis on complex requirements identified by the Decider. It breaks down high-level tasks into detailed execution plans for the Implementer.

## Purpose

Not all requirements can be implemented directly. Some require thoughtful architecture, multi-step execution, or careful consideration of side effects. The Planner bridges the gap between "what needs to be done" (Decider) and "how to do it" (Implementer).

## Inputs

- **Requirements**: YAML files in `.jules/exchange/requirements/` marked with `requires_deep_analysis: true`.
- **Codebase Context**: Full access to read the repository to understand dependencies and current implementation.

## Outputs

- **Expanded Requirement**: Updates the original requirement file in `.jules/exchange/requirements/` with:
  - `analysis`: A detailed breakdown of the problem.
  - `plan`: A step-by-step execution plan for the Implementer.
  - `files_to_touch`: List of files likely to be modified.
  - `verification_plan`: How to verify the changes.
  - `requires_deep_analysis`: Set to `false` (marking it ready for implementation).

## Execution

The Planner is executed via the CLI, typically pointing to a specific requirement:

```bash
jlo run planner .jules/exchange/requirements/<requirement-id>.yml
```

Alternatively, the automated workflow finds requirements needing analysis and runs the Planner on them.

### Process Flow

1. **Requirement Analysis**:
   - Reads the input requirement.
   - Analyzes the codebase relevant to the requirement's `affected_areas`.

2. **Plan Generation**:
   - Decomposes the task into logical steps.
   - Identifies risks and edge cases.
   - Formulates a verification strategy (e.g., new tests).

3. **Update Requirement**:
   - Rewrites the requirement file with the added `plan` and `analysis` sections.
   - Clears the `requires_deep_analysis` flag.

## Tasks

The Planner uses tasks defined in `.jules/layers/planner/tasks/` to structure its reasoning. The main prompt, `planner_prompt.j2`, includes these tasks to guide the agent.

## Troubleshooting

- **Planner Skipped**: If `requires_deep_analysis` is `false`, the Planner will skip the requirement.
- **Incomplete Plans**: If the LLM context window is exceeded, the plan might be truncated. Check the requirement file for completeness.
