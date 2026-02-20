# Documentation

This directory contains key architectural decisions and operational guides.

## Structure

### [Architecture](architecture/)

High-level design and constraints.

- [Boundary & Dependency Model](architecture/ARCHITECTURE_BOUNDARY.md)
- [Control Plane Ownership](architecture/CONTROL_PLANE_OWNERSHIP.md)
- [Prompt Assembly Policy](architecture/PROMPT_ASSEMBLY.md)
- [Scaffold Design](architecture/scaffold.md)

### [Operations](operations/)

Operational guides and reference maps.

- [Setup Guide](operations/SETUP_GUIDE.md)
- [Template System](operations/templates.md)
- [Workflow Branch Impact Map](operations/WORKFLOW_BRANCH_IMPACT_MAP.md)
- [Workflow Layer Change Map](operations/WORKFLOW_LAYER_CHANGE_MAP.md)

#### [Role Guides](operations/layers/)

Specific guides for each architectural role.

- [Decider](operations/layers/DECIDER.md)
- [Implementer](operations/layers/IMPLEMENTER.md)
- [Innovators](operations/layers/INNOVATORS.md)
- [Integrator](operations/layers/INTEGRATOR.md)
- [Narrator](operations/layers/NARRATOR.md)
- [Observers](operations/layers/OBSERVERS.md)
- [Planner](operations/layers/PLANNER.md)

## Development Context

Core development and design context located in the source tree.

- [CLI Development](../src/AGENTS.md) — Rust CLI development context (SSOT for verification)
- [GitHub Workflows](../.github/AGENTS.md) — GitHub Actions workflows design

## Keeping Docs Updated

- Code Changes: If you modify logic that affects architecture or usage, update the corresponding doc.
- New Features: Add new guides or update existing ones.
- Reference: Link to these docs from code comments or other documentation where relevant.
