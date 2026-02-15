# Documentation

This directory contains key architectural decisions, role guides, and operational manuals.

## Purpose

- **Architecture**: Define the high-level design and constraints (e.g., control plane ownership).
- **Guides**: Provide instructions for specific roles (Innovator, Observer, etc.) and tasks.
- **Maintenance**: Keep documentation in sync with code changes.

## Structure

### Core Documentation

| File | Purpose |
|------|---------|
| `AGENTS.md` | Overview of the documentation structure (this file). |
| `ARCHITECTURE_BOUNDARY.md` | Canonical boundary vocabulary, ownership, and dependency model for `src/`. |
| `CONTROL_PLANE_OWNERSHIP.md` | Defines the ownership model for `.jlo/` (control plane) and `.jules/` (runtime state). |
| `PROMPT_ASSEMBLY.md` | Details how prompts are constructed and managed (contracts + tasks). |
| `REPRODUCTION_GUIDE.md` | Steps to reproduce issues or environments. |
| `WORKFLOW_BRANCH_IMPACT_MAP.md` | Details how branches affect workflow execution. |
| `WORKFLOW_LAYER_CHANGE_MAP.md` | Maps code changes to specific workflow layers. |

### Role Guides (`layers/`)

Specific guides for each architectural role are located in `docs/layers/`.

| File | Role |
|------|------|
| `layers/DECIDER.md` | Guide for the Decider role (triage & decision making). |
| `layers/IMPLEMENTER.md` | Guide for the Implementer role (execution & code changes). |
| `layers/INNOVATORS.md` | Guide for the Innovator role (feature proposals & refinement). |
| `layers/NARRATOR.md` | Guide for the Narrator role (context & continuity). |
| `layers/OBSERVERS.md` | Guide for the Observer role (monitoring & feedback). |
| `layers/PLANNER.md` | Guide for the Planner role (strategy & decomposition). |

## Keeping Docs Updated

- **Code Changes**: If you modify logic that affects architecture or usage, update the corresponding doc.
- **New Features**: Add new guides or update existing ones.
- **Reference**: Link to these docs from code comments or other documentation where relevant.
