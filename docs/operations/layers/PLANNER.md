# Planner
Planning detail agent for requirements not yet implementation-ready.

## Interface
- Input: Requirements in `.jules/exchange/requirements/` where `implementation_ready: false`.
- Output: Details decider parameters (`affected_areas`, `constraints`, `risks`, `acceptance_criteria`, `verification_criteria`) and sets `implementation_ready: true`.
- Execution: `jlo run planner <requirement-path>`

## Constraints
- Scope: Modifies `.jules/exchange/requirements/` only. Reads entire repo.
- Pre-condition: Must have a `planner_request_reason`.

## Logic
1. Analysis: Read requirement and relevant codebase area.
2. Decomposition: Break task into steps, identify risks, and formulate verification.
3. Update: Expand existing requirement parameters and set `implementation_ready` to true.

## Resources
- Tasks:
  - expand_requirement.yml: Logic for deep analysis and plan generation.
- Prompt: `planner_prompt.j2`
