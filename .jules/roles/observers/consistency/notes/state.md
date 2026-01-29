# Consistency State

## Identified Inconsistencies

### Missing Implementers Layer Support
- **Status:** Open
- **Description:** The `Implementers` layer is documented as a core architectural component but is missing from the `Layer` domain model and scaffolding templates.
- **Evidence:** `README.md`, `AGENTS.md`, `src/domain/layer.rs`
- **Impact:** `jlo` cannot manage implementer roles.

### Incorrect Documentation on ROLE_NAME Replacement
- **Status:** Open
- **Description:** `AGENTS.md` incorrectly states that `jlo` performs string replacement for `ROLE_NAME`. The implementation preserves placeholders.
- **Evidence:** `AGENTS.md`, `src/services/role_template_service.rs`
- **Impact:** Misleading documentation for contributors.
