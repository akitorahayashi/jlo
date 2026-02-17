# Developer Workflow

See [src/AGENTS.md](src/AGENTS.md) for development verification commands and CLI architecture details.

## Documentation Index

### Core Guides
- [src/AGENTS.md](src/AGENTS.md) — Rust CLI development context (SSOT for verification)
- [.github/AGENTS.md](.github/AGENTS.md) — GitHub Actions workflows design
- [src/assets/scaffold/AGENTS.md](src/assets/scaffold/AGENTS.md) — `.jules/` scaffold design
- [src/assets/templates/AGENTS.md](src/assets/templates/AGENTS.md) — Template system

### Architectural Guides
- [Control Plane Ownership](docs/architecture/CONTROL_PLANE_OWNERSHIP.md) — `.jlo/` vs `.jules/` ownership model and projection rules
- [Prompt Assembly Policy](docs/architecture/PROMPT_ASSEMBLY.md) — Policy for `contracts.yml` + `tasks/*.yml` assembly

### Operational Guides
- [Reproduction Guide](docs/operations/REPRODUCTION_GUIDE.md) — How to reproduce the Jules workflow in other projects
- [Workflow Branch Impact Map](docs/operations/WORKFLOW_BRANCH_IMPACT_MAP.md) — Operational index for branch-contract changes
- [Workflow Layer Change Map](docs/operations/WORKFLOW_LAYER_CHANGE_MAP.md) — Repository touch points for layer-level changes
