# Innovator Role Guide

The **Innovators** layer generates improvement proposals from repository context and persona-specific workstation memory.

## Inputs

- Repository codebase context
- Workstation perspective: `.jules/workstations/<persona>/perspective.yml`
- Layer/role contracts under `.jules/layers/innovators/` and `.jlo/roles/innovators/`

## Outputs

- Three proposal files:
  - `.jules/exchange/proposals/<persona>-<kebab-case-slug>.yml`
- Updated workstation perspective:
  - `.jules/workstations/<persona>/perspective.yml`

Idea/comment intermediate artifacts are not used.

## Execution

```bash
jlo run innovators --role <persona> --task create_three_proposals
```

Example:

```bash
jlo run innovators --role recruiter --task create_three_proposals
```

## Task Contract

- Task file: `.jules/layers/innovators/tasks/create_three_proposals.yml`
- Each run must emit three directionally distinct proposals.
- Proposal filenames must include persona prefix + slug to avoid collisions.
- `recent_proposals` in workstation perspective must be updated with emitted titles.

## Proposal Schema

Proposal schema is defined by:

- `.jules/layers/innovators/schemas/proposal.yml`

Key fields:

- `id`
- `persona`
- `created_at`
- `title`
- `problem`
- `introduction`
- `importance`
- `impact_surface`
- `implementation_cost`
- `consistency_risks`
- `verification_signals`
