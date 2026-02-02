# Refactor Policy (Prompt Assembly + Scaffold Layout)

This repository treats the embedded scaffold under `src/assets/scaffold/` as the authoritative source of truth for the `.jules/` workspace layout, prompt composition inputs, and artifact schema locations.

## Asset Authority

- Scaffold and prompt assets exist as real files under `src/assets/` and are embedded via `include_dir!`.
- Rust source does not embed scaffold or prompt file contents as string constants.
- All user-facing lists (layers, supported scopes, schema-derived enums) are derived from authoritative sources (code enums and/or the embedded asset tree) rather than duplicated in documentation or CLI output.

## Workspace Layout (Collision-Free)

Layer directories under `.jules/roles/<layer>/` are structured so that role directories do not collide with layer-owned directories.

- Multi-role layers place role directories under `.jules/roles/<layer>/roles/<role>/`.
- Layer-owned files and directories are siblings of `roles/`, not siblings of role directories.
- Artifact schemas live under `.jules/roles/<layer>/schemas/` and are referenced by contracts and validation tooling.

## Prompt Assembly (Asset-Driven)

Prompt composition is specified by `.jules/roles/<layer>/prompt_assembly.yml`.

- `prompt_assembly.yml` is parsed as YAML and treated as a declarative assembly specification.
- Runtime context requirements are declared in `prompt_assembly.yml` and validated before any file inclusion occurs.
- Includes are concatenated in declared order with deterministic section boundaries and titles.
- Missing required inputs fail fast. Optionality is explicit and visible as “skipped” rather than silently substituted.

## Templating Semantics (Jinja-Compatible, YAML-Preserved)

Templating is limited to string interpolation within YAML field values.

- Files remain `.yml` to preserve YAML tooling ergonomics and to keep the assembly specification readable as YAML.
- Rendering uses a Jinja-compatible engine with strict undefined behavior for declared runtime variables.
- Template control structures are not part of the prompt assembly surface area; the assembly specification remains declarative rather than executable.

## Update and Migration Semantics

- Managed file selection is derived from the embedded scaffold tree rather than hardcoded path lists.
- Workspace migrations are explicit: layout transitions are detected, reported, and applied with observable outcomes and backups.
- No silent fallbacks exist for missing required files or ambiguous layouts; failures are surfaced as errors.

## Validation Expectations

The repository’s validation tooling treats the scaffold layout and schema locations as authoritative:

- Structural validation checks the expected directory layout and required layer-owned files.
- Schema validation derives enum constraints from schema files located under the scaffold-defined schema directory.
