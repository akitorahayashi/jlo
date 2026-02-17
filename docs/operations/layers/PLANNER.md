# Planner
Deep analysis agent for complex requirements.

## Interface
- Input: Requirements in `.jules/exchange/requirements/` where `requires_deep_analysis: true`.
- Output: Updates the requirement file with `analysis`, `plan`, `files_to_touch`, and `verification_plan`. Sets `requires_deep_analysis: false`.
- Execution: `jlo run planner <requirement-path>`

## Constraints
- Scope: Modifies `.jules/exchange/requirements/` only. Reads entire repo.
- Pre-condition: Must have a `deep_analysis_reason`.

## Logic
1. Analysis: Read requirement and relevant codebase area.
2. Decomposition: Break task into steps, identify risks, and formulate verification.
3. Update: Inject `plan` and `analysis` into the source YAML; clear the deep analysis flag.

## Resources
- Tasks:
  - expand_requirement.yml: Logic for deep analysis and plan generation.
- Prompt: `planner_prompt.j2`
