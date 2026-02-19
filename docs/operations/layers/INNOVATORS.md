# Innovators
Layer generating improvement proposals from repository analysis.

## Interface
- Input: Repo context, role contracts from `.jlo/roles/innovators/<role>/role.yml`, layer contracts.
- Output: 3 proposals per run at `.jules/exchange/proposals/<role>-<slug>.yml`.
- Execution: `jlo run innovators --role <role> --task create_three_proposals`

## Constraints
- Scope: Modifies `.jules/exchange/proposals/`. Reads entire repo.
- Quantity: Exactly 3 proposals must be emitted.
- Naming: `<role>-<kebab-case-slug>.yml`.
- Parallelism: Runs independently of the Narrator -> Observer -> Decider flow.
- Role policy: `role.yml` `profile` defines ideation lens and `constraint` defines prohibited or required boundaries.

## Management
- Bootstrap: `jlo workflow bootstrap managed-files` materializes managed `.jules/` assets from embedded scaffold.

## Resources
- Schema: `.jules/schemas/innovators/proposal.yml`
- Tasks:
  - create_three_proposals.yml: Generates directionally distinct proposals.
