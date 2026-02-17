# Documentation

This directory contains key architectural decisions and operational guides.

## Structure

### [Architecture](architecture/)

High-level design and constraints.

- [Boundary & Dependency Model](architecture/ARCHITECTURE_BOUNDARY.md)
- [Control Plane Ownership](architecture/CONTROL_PLANE_OWNERSHIP.md)
- [Prompt Assembly Policy](architecture/PROMPT_ASSEMBLY.md)

### [Operations](operations/)

Operational guides and reference maps.

- [Reproduction Guide](operations/REPRODUCTION_GUIDE.md)
- [Workflow Branch Impact Map](operations/WORKFLOW_BRANCH_IMPACT_MAP.md)
- [Workflow Layer Change Map](operations/WORKFLOW_LAYER_CHANGE_MAP.md)

#### [Role Guides](operations/layers/)

Specific guides for each architectural role.

- [Decider](operations/layers/DECIDER.md)
- [Implementer](operations/layers/IMPLEMENTER.md)
- [Innovators](operations/layers/INNOVATORS.md)
- [Narrator](operations/layers/NARRATOR.md)
- [Observers](operations/layers/OBSERVERS.md)
- [Planner](operations/layers/PLANNER.md)

## Keeping Docs Updated

- Code Changes: If you modify logic that affects architecture or usage, update the corresponding doc.
- New Features: Add new guides or update existing ones.
- Reference: Link to these docs from code comments or other documentation where relevant.
