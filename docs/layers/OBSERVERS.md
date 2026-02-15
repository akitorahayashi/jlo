# Observer Role Guide

The **Observers** layer consists of specialized roles that continuously analyze the codebase, detecting issues, ensuring consistency, and providing feedback on proposed changes. Unlike singular agents (like Decider or Planner), Observers operate as a collection of distinct personas, each with a specific domain of expertise.

## Purpose

The primary goal of Observers is to maintain the health and integrity of the project. They act as automated auditors and reviewers, surfacing problems (bugs, architectural violations, documentation gaps) and providing constructive feedback on new ideas from Innovators.

## Roles

Observers are categorized by their area of focus:

- **Architecture**:
  - `data_arch`: Focuses on data models, schemas, and persistence.
  - `structural_arch`: Ensures adherence to project structure and module boundaries.
- **Documentation**:
  - `codifier`: Transforms recurring patterns into formal skills and procedures.
  - `consistency`: Checks for terminology and style consistency across docs and code.
  - `librarian`: Manages documentation organization and information architecture.
  - `tactician`: Reviews tactical implementation details against strategic goals.
- **Experience**:
  - `cli_sentinel`: Enforces CLI standards and user experience consistency.
  - `ui_designer`: Reviews UI/UX aspects (where applicable).
- **Language**:
  - `gopher`, `pythonista`, `rustacean`, `swifter`, `typescripter`: Language-specific experts that enforce idioms and best practices.
  - `taxonomy`: Standardizes naming conventions and terminology.
- **Operations**:
  - `devops`: Focuses on CI/CD, build processes, and infrastructure.
  - `observability`: Ensures proper logging, metrics, and tracing.
- **Testing**:
  - `cov`: Analyzes test coverage and gaps.
  - `qa`: Focuses on functional correctness and test scenarios.

## Inputs

- **Codebase State**: The current state of the repository (source code, configuration, documentation).
- **Innovator Ideas**: `idea.yml` files in `.jules/exchange/innovators/<persona>/` (when reviewing proposals).

## Outputs

- **Events**: YAML files located in `.jules/exchange/events/pending/`. These represent issues or findings that need attention.
- **Comments**: YAML files located in `.jules/exchange/innovators/<persona>/comments/`. These are feedback on specific Innovator ideas.

## Execution

Observers are executed via the CLI, specifying the role:

```bash
jlo run observers --role <role_name>
```

Example:
```bash
jlo run observers --role rustacean
```

### Process Flow

1. **Analysis**:
   - The observer scans the codebase relevant to its domain.
   - If an Innovator has an active `idea.yml`, the observer reviews it for risks or improvements.

2. **Finding Generation**:
   - **Events**: If an issue is found (e.g., a broken link, a violation of style), an event file is created.
   - **Comments**: If reviewing an idea, a comment file is created with feedback (e.g., "This contradicts our architectural principles").

3. **Submission**:
   - The observer commits the generated files (events or comments) to a new branch.
   - A Pull Request is created to submit these findings for the Decider to process.

## Schemas

### Event Schema (`observer_event.yml`)
Represents a finding or issue.
- `id`: Unique identifier.
- `confidence`: `high`, `medium`, or `low`.
- `title`: Short summary of the issue.
- `statement`: Detailed description.
- `evidence`: List of file paths and locations supporting the finding.

### Comment Schema (`observer_comment.yml`)
Represents feedback on an Innovator's idea.
- `author`: The observer role (e.g., `rustacean`).
- `type`: `risk`, `alternative`, `supplement`, or `question`.
- `target`: The specific part of the idea being addressed.
- `summary`: High-level feedback.
- `rationale`: Why this feedback matters.

## Troubleshooting

- **No Events Created**: The observer might not have found any issues, or the issue might be below the confidence threshold.
- **Bridge Task Missing**: If reviewing ideas, ensure the `bridge_comments.yml` task is available in the observer's configuration.
