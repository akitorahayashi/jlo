# Terminology Analysis

## Layer Terminology
- **Code Representation**: `Layer` enum (singular).
- **Directory Names**: Plural (`observers`, `deciders`, `planners`, `implementers`).
  - Defined in `Layer::dir_name()`.
  - Matched by scaffold structure: `.jules/roles/{plural_layer}/`.
- **Display Names**: Singular (`Observer`, `Decider`, `Planner`, `Implementer`).
  - Defined in `Layer::display_name()`.
- **Inconsistency**: `src/assets/templates/layers/` uses singular names (`observer`, `decider`...) which contradicts the plural convention used elsewhere for directories.

## Role Terminology
- **Concept**: A specific agent specialization within a layer.
- **Identifier**: `RoleId` (alphanumeric, hyphens, underscores).
- **Representation**:
  - `RoleId` struct in `src/domain/role_id.rs`.
  - `String` in `DiscoveredRole` (potential type safety issue).
  - `&str` in `RoleTemplateStore` port methods.

## Workspace Structure
- **Root**: `.jules/`
- **Roles**: `.jules/roles/{layer}/{role_id}/`
- **Feedbacks**: `.jules/roles/observers/{role_id}/feedbacks/` (Specific to Observers).
- **Notes**: `.jules/roles/{layer}/{role_id}/notes/`.

## Files
- `role.yml`: Defines the role's specific focus (Observers only).
- `prompt.yml`: Defines the role's execution instructions (All roles).
- `contracts.yml`: Defines the layer's shared behavior (Static).
