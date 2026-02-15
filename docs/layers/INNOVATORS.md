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

## Execution & Lifecycle

### Workstation Management

Workstation state (`perspective.yml`) is managed by the `bootstrap` command. Individual agents are prohibited from self-initializing their perspectives.

- **Authority**: `src/app/commands/workflow/bootstrap.rs`
- **Policy**: "Every scheduled role has an environment; no unscheduled role environment remains."
- **Workflow**: 
  1. `bootstrap` reads `.jlo/config.toml` (`[observers].roles`, `[innovators].roles`) as the Source of Truth.
  2. **Ensure**: Missing workstation directories/perspectives are created from layer schemas.
  3. **Prune**: Workstation directories for roles no longer in the schedule are recursively deleted.

### Parallel Execution

Innovators run independently from the main decision flow (Narrator -> Observers -> Decider).

- **Workflow Trigger**: `jules-run-innovators.yml` supports both `workflow_dispatch` and `workflow_call`.
- **Concurrency**: Innovator jobs run in parallel with the Narrator phase to maximize throughput.

### Task Contract

```bash
jlo run innovators --role <persona> --task create_three_proposals
```

- **Task file**: `.jules/layers/innovators/tasks/create_three_proposals.yml`
- **Output**: Each run emits **three** directionally distinct proposals to `.jules/exchange/proposals/<persona>-<slug>.yml`.
- **Collisions**: Filenames MUST use the persona-slug combination.
- **Memory**: The persona's `perspective.yml` must be updated with emitted proposal metadata.

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
