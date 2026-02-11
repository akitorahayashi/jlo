# Documentation

This directory contains key architectural decisions, role guides, and operational manuals.

## Purpose

- **Architecture**: Define the high-level design and constraints (e.g., control plane ownership).
- **Guides**: Provide instructions for specific roles (Innovator, Observer) and tasks.
- **Maintenance**: Keep documentation in sync with code changes.

## Structure

| File | Purpose |
|------|---------|
| `CONTROL_PLANE_OWNERSHIP.md` | Defines the ownership model for `.jlo/` (control plane) and `.jules/` (runtime state). |
| `WORKFLOW_LAYER_CHANGE_MAP.md` | Maps code changes to specific workflow layers. |
| `WORKFLOW_BRANCH_IMPACT_MAP.md` | Details how branches affect workflow execution. |
| `INNOVATOR_ROLE_YML_GUIDE.md` | Guide for configuring Innovator roles. |
| `OBSERVER_ROLE_YML_GUIDE.md` | Guide for configuring Observer roles. |
| `REPRODUCTION_GUIDE.md` | Steps to reproduce issues or environments. |

## Keeping Docs Updated

- **Code Changes**: If you modify logic that affects architecture or usage, update the corresponding doc.
- **New Features**: Add new guides or update existing ones.
- **Reference**: Link to these docs from code comments or other documentation where relevant.
