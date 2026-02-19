# Innovators
Layer generating improvement proposals based on workstation perspectives.

## Interface
- Input: Repo context, `.jules/workstations/<role>/perspective.yml`, layer contracts.
- Output: 3 proposals per run at `.jules/exchange/proposals/<role>-<slug>.yml`, updated `perspective.yml`.
- Execution: `jlo run innovators --role <role> --task create_three_proposals`

## Constraints
- Scope: Modifies `.jules/exchange/proposals/` and `.jules/workstations/`. Reads entire repo.
- Quantity: Exactly 3 proposals must be emitted.
- Naming: `<role>-<kebab-case-slug>.yml`.
- Parallelism: Runs independently of the Narrator -> Observer -> Decider flow.
- Memory quality: `perspective.yml` remains durable analytical memory; avoid per-run chronology and ID-based tracking text.

## Management
- Bootstrap: `jlo workflow bootstrap` initializes and prunes `perspective.yml` based on `.jlo/config.toml`. Agents never self-initialize.
- Memory: Metadata for emitted proposals must be stored in the role's `perspective.yml`.

## Resources
- Schema: `.jules/schemas/innovators/proposal.yml`
- Tasks:
  - prepare_ideation_scope.yml: Reads role memory and defines ideation scope.
  - create_three_proposals.yml: Generates directionally distinct proposals.
  - refresh_innovator_perspective.yml: Updates durable innovator memory for the next run.
