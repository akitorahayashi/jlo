# Prompt Assembly Policy: `contracts.yml` + `tasks/*.yml`

## Objective
Prompt composition is declarative and deterministic.
Layer-level rules are separated from task-level instructions.

## File Model
- `src/assets/prompt-assemble/<layer>/contracts.yml` (embedded)
- `src/assets/prompt-assemble/<layer>/tasks/<task-id>.yml` (embedded)
- `src/assets/prompt-assemble/<layer>/<layer>_prompt.j2` (embedded)

## Responsibility Split
- `contracts.yml` describes layer scope and non-negotiable rules shared across all tasks.
  - Example: do not edit source code, writable paths, advisory-only interpretation of change summary.
- `tasks/*.yml` describes independent task units.
  - Example: max 3 events in one run, required evidence fields, output path for that task.

## Deterministic Assembly Rules
- Branching and ordering are defined only in `<layer>_prompt.j2`.
- `j2` guarantees deterministic behavior when:
  - include paths are explicit (no directory scan/glob ordering),
  - conditions are deterministic (`file_exists(...)` and injected context vars),
  - include order is written explicitly in template.
- Missing required task files fail fast (`include_required`), no silent fallback.

## Ordering Rule
- Task order is the include order in `<layer>_prompt.j2`.
- If two tasks are independent, either order is acceptable; choose one explicit order and keep it stable.
- No merged `workflow` array is used for cross-file sequencing.

## Naming Rule
- No generic names such as `base`, `core`, `utils`, or `helpers`.
- Task files are named by concrete intent (e.g., `detect_findings.yml`, `emit_events.yml`, `comment_idea.yml`).

## Changes Output Path
- Target: `.jules/exchange/changes.yml` (narrator output, single overwrite-in-place file).
- Legacy: `.jules/changes/latest.yml` — will be migrated to the target path in Phase 2.

## Scaffold Source Conventions
- Prompt-assembly assets live in `src/assets/prompt-assemble/` and are embedded via `include_dir!`.
- Schemas are deployed to `.jules/schemas/<layer>/` from `src/assets/scaffold/jules/schemas/`.
- Path mapping is performed by the `map_scaffold_path` / `unmap_scaffold_path` functions in the adapter layer.
