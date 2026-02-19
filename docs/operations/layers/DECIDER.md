# Decider
Triage center converting Observer events into requirements.

## Interface
- Input: `.jules/exchange/events/pending/*.yml`, `.jules/exchange/changes.yml`
- Output: `.jules/exchange/requirements/<kebab-id>.yml`, moves processed events to `.jules/exchange/events/decided/`
- Execution: `jlo run decider`

## Constraints
- Scope: Reads repo; modifies `.jules/exchange/requirements/` and `events/`.
- Identity: Stable kebab-case filenames.
- Planning Gate: If `implementation_ready` is false, `planner_request_reason` must be provided.
- Verification: `verification_criteria` must be actionable text.

## Logic
1. Discovery: Scan `pending/` events.
2. Triage: Group similar events, filter noise, assign priority (`low|medium|high`) and label.
3. Generation: Create requirement with `requirement_id` mapping to source events.
4. Cleanup: Archive source events to `decided/`.

## Resources
- Schema: `.jules/schemas/decider/requirements.yml`
- Tasks:
  - triage.yml: Logic for event analysis and grouping.
- Prompt: `decider_prompt.j2`
