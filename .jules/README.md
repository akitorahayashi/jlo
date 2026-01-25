# .jules/

The `.jules/` directory maintains organizational memory and records of work in this repository.
It is designed so that agents executed from the scheduler and humans can understand the state of the project in a consistent structure.

## Structure

```
.jules/
  README.md           # This file
  .jo-version         # Version information of the tool that manages this structure
  roles/              # Role-specific work directories
    <role>/
      prompt.yml      # Role-specific prompt instructions (pasteable as-is)
      reports/        # Accumulation location for execution reports
        YYYY-MM-DD_HHMMSS.md
```

## Roles

Each role analyzes and evaluates the repository from an independent perspective.

- `prompt.yml`: Describes the purpose of the role and the viewpoints to consider during execution (pasteable as-is)
- `reports/`: Accumulates execution result reports in date-time stamped files

## Execution Model

Each task executed from the scheduler performs the following operations:

1. Read `AGENTS.md` to understand project constraints and policies
2. Read the latest few items from the role's `reports/` to grasp past analyses
3. Conduct new analysis and output to `.jules/roles/<role>/reports/YYYY-MM-DD_HHMMSS.md`
