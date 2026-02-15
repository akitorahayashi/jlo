# Jules Contract

This file defines the binding rules for Jules agents operating in this repository.

## Authority

- This file is authoritative for global rules and shared conventions.
- Each layer contract is authoritative for layer-specific rules and schemas:
  - `.jules/layers/narrator/contracts.yml`
  - `.jules/layers/observers/contracts.yml`
  - `.jules/layers/decider/contracts.yml`
  - `.jules/layers/planner/contracts.yml`
  - `.jules/layers/implementer/contracts.yml`
  - `.jules/layers/integrator/contracts.yml`
  - `.jules/layers/innovators/contracts.yml`

- **Role Definitions**: Defined in `.jlo/` (Control Plane).
  - `.jlo/roles/<layer>/roles/<role>/role.yml`

If a required contract file is missing or conflicts with another contract, execution stops and the
conflict is reported.

## Required Read Order

1. `.jules/JULES.md`
2. The layer `contracts.yml` (or phase-specific contract for innovators)
3. Role-specific inputs required by the layer contract

## Changes Feed

The Narrator layer produces `.jules/exchange/changes.yml`, summarizing recent codebase changes.

- `.jules/exchange/changes.yml` is overwritten in-place (no time-series).
- Narrator excludes `.jules/` from all diffs and path lists.
- Observers may use this only as a secondary hint after baseline repository inspection.
- Schema is defined by `.jules/layers/narrator/schemas/changes.yml`.

## Exchange Model

Jules uses a flat exchange model for handing off events and requirements between layers. The exchange is located in `.jules/exchange/`.

- **Events** (Observer output, Decider input):
  - `.jules/exchange/events/<state>/*.yml` (states: `pending`, `decided`)
- **Requirements** (Decider/Planner output, Implementer input):
  - `.jules/exchange/requirements/*.yml`
- **Innovator Rooms**:
  - `.jules/exchange/innovators/<persona>/` (contains proposals and comments)

## Repository Data Flow

The pipeline is file-based and uses local requirements as the handoff point:

`narrator -> observers -> decider -> [planner] -> implementer`

Narrator runs first, producing `.jules/exchange/changes.yml` as a secondary hint for observer triage.

After decider output:
- Requirements with `requires_deep_analysis: false` are ready for implementation.
- Requirements with `requires_deep_analysis: true` trigger deep analysis by planner.
- Implementer is invoked via `jlo run implementer` with a local requirement file. Scheduled workflows may dispatch implementer according to repository policy.

## Requirement Identity and Deduplication

- Requirement filenames use stable kebab-case identifiers, not dates (e.g. `auth-inconsistency.yml`).
- Observers check existing requirements before emitting events to avoid duplicates.
- Decider links related events to requirements (populating `source_events` in the requirement).
- Events are preserved in the exchange until an implementation workflow removes them.

## Deep Analysis

When a requirement requires deep analysis:
- `requires_deep_analysis: true` must have a non-empty `deep_analysis_reason` field.
- Planner expands the requirement and sets `requires_deep_analysis: false`.
- The original rationale is preserved and expanded with findings.

## File Rules

- YAML only (`.yml`) and English only.
- Artifacts are created by copying the corresponding schema and filling its fields:
  - Changes: `.jules/layers/narrator/schemas/changes.yml`
  - Events: `.jules/layers/observers/schemas/event.yml`
  - Requirements: `.jules/layers/decider/schemas/requirements.yml`

## Git And Branch Rules

The runner provides `starting_branch`. Agents do not change it.

Branch names:

- Narrator: `jules-narrator-<id>`
- Observers: `jules-observer-<id>`
- Decider: `jules-decider-<id>`
- Planner: `jules-planner-<id>`
- Implementer: `jules-implementer-<label>-<id>-<short_description>`
- Integrator: `jules-integrator-<timestamp>-<id>`

`<id>` is 6 lowercase alphanumeric characters unless the layer contract specifies otherwise.

`<label>` is a requirement label defined in `.jules/github-labels.json` (e.g., `bugs`, `feats`).

## Safety Boundaries

- Narrator modifies only `.jules/exchange/changes.yml`.
- Observers, Decider, and Planner modify only `.jules/`.
- Implementer modifies only what the requirement specifies, runs the verification command, then
  creates a pull request for human review.
- Integrator merges implementer branches contextually and creates a single integration pull request
  for human review.

## Forbidden By Default

- `.github/workflows/` is not modified unless explicitly required by the requirement.
