# Terminology State Analysis

## Issues Identified

### 1. Missing `Implementers` Layer in Domain Model
- **Observation**: The `Layer` enum in `src/domain/layer.rs` defines `Observers`, `Deciders`, and `Planners`, but is missing `Implementers`.
- **Impact**: Inconsistency with `AGENTS.md`, `.jules/JULES.md`, and CI workflows which clearly define an Implementer layer.
- **Evidence**: `src/domain/layer.rs`, `src/domain/error.rs`.

### 2. Dead Code: `merger` Templates
- **Observation**: The directory `src/assets/templates/layers/merger/` exists, but its `prompt.yml` states it has been removed/retired.
- **Impact**: Unnecessary noise in the codebase and confusion about whether the layer is active.
- **Evidence**: `src/assets/templates/layers/merger/prompt.yml`.

### 3. Layer Naming Inconsistency (Singular vs Plural)
- **Observation**:
    - `src/assets/templates/layers/` uses **singular** names: `decider`, `implementer`, `observer`, `planner`.
    - `.jules/roles/` uses **plural** names: `deciders`, `implementers`, `observers`, `planners`.
    - `Layer` enum uses **plural** variants (`Observers`) and `dir_name()` returns plural strings.
- **Impact**: Confusing directory structure mapping. The template lookup logic must handle this discrepancy.
- **Evidence**: `src/assets/templates/layers/`, `.jules/roles/`, `src/domain/layer.rs`.
