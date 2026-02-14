# Documentation

This directory contains key architectural decisions, role guides, and operational manuals.

## Purpose

- **Architecture**: Define the high-level design and constraints (e.g., control plane ownership).
- **Operations**: Provide operational guides, reproduction steps, and impact maps.
- **Layers**: Specific documentation for each layer.

## Structure

### Architecture (`docs/architecture/`)
- [Control Plane Ownership](architecture/CONTROL_PLANE_OWNERSHIP.md) — Defines the ownership model for `.jlo/` (control plane) and `.jules/` (runtime state).

### Operations (`docs/operations/`)
- [Workflow Branch Impact Map](operations/WORKFLOW_BRANCH_IMPACT_MAP.md) — Details how branches affect workflow execution.
- [Workflow Layer Change Map](operations/WORKFLOW_LAYER_CHANGE_MAP.md) — Maps code changes to specific workflow layers.
- [Reproduction Guide](operations/REPRODUCTION_GUIDE.md) — Steps to reproduce issues or environments.

### Layers (`docs/layers/`)
Documentation for specific layers (Narrator, Decider, etc.).

## Keeping Docs Updated

- **Code Changes**: If you modify logic that affects architecture or usage, update the corresponding doc.
- **New Features**: Add new guides or update existing ones.
- **Reference**: Link to these docs from code comments or other documentation where relevant.
