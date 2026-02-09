# Workflow Layer Change Map

This document defines the current repository touch points for layer-level changes in `jlo`.

A layer-level change means one of the following:
- Adding a new layer.
- Removing an existing layer.
- Changing a layer type (single-role, multi-role, issue-driven).
- Changing layer artifacts used by workflow execution and validation.

## Authoritative Sources
- Layer model: `src/domain/workspace/layer.rs`
- Control-plane ownership: `docs/CONTROL_PLANE_OWNERSHIP.md`
- CLI entry points: `src/app/cli/run.rs`, `src/app/cli/workflow.rs`
- Runtime dispatch: `src/app/commands/run/mod.rs`, `src/app/commands/workflow/run/layer_executor.rs`
- Schedule model: `src/domain/configuration/schedule.rs`
- Scaffold assets: `src/assets/scaffold/.jules/**`
- Workflow templates: `src/assets/workflows/.github/workflows/*.j2`
- Mock execution: `src/app/commands/run/mock/**`
- Mock recovery cleanup: `src/app/commands/workflow/cleanup/mock.rs`

## Layer Change Surface

| Surface | Current coupling | Main files |
|---|---|---|
| Layer identity and parsing | `Layer::ALL`, `dir_name`, `from_dir_name`, type flags are hard-coded | `src/domain/workspace/layer.rs` |
| CLI command shape | `run` subcommands are explicit per layer | `src/app/cli/run.rs` |
| Workflow command parsing | `workflow run <layer>` relies on `Layer::from_dir_name` | `src/app/cli/workflow.rs` |
| Runtime execution branch | Main `run` path and workflow layer executor match on layer enum | `src/app/commands/run/mod.rs`, `src/app/commands/workflow/run/layer_executor.rs` |
| Multi-role scheduling | `scheduled.toml` currently models observer/decider roles explicitly | `src/domain/configuration/schedule.rs`, `src/adapters/workstream_schedule_filesystem.rs` |
| Prompt and contracts | Layer assets are file-based and embedded with `include_dir` | `src/assets/scaffold/.jules/roles/<layer>/**` |
| Doctor validation | Structural/schema/semantic checks iterate layers and workstream data contracts | `src/app/commands/doctor/*.rs` |
| Matrix/routing orchestration | Workflow matrix/run logic assumes current layer set | `src/app/commands/workflow/matrix/*.rs`, `src/app/commands/workflow/run/*.rs` |
| Workflow orchestration | Layer sequence is defined in workflow templates | `src/assets/workflows/.github/workflows/jules-workflows.yml.j2` |
| Auto-merge qualification | Branch prefixes are a static allowed list matching the Layer model | `src/assets/workflows/.github/workflows/jules-automerge.yml.j2` |
| Mock behavior | Per-layer mock behavior is implemented in dedicated modules | `src/app/commands/run/mock/*.rs` |
| Failure recovery | Mock residue cleanup scope is explicit and code-defined | `src/app/commands/workflow/cleanup/mock.rs` |
| Tests | Integration tests assert layer structure, workflow text, and mock behavior | `tests/cli_flow.rs`, `tests/cli_commands.rs`, `tests/workflow_kit.rs`, `tests/mock_mode.rs` |

## Layer Types and Downstream Effects

| Layer trait change | Downstream code that changes |
|---|---|
| Add/remove layer in enum | `Layer::ALL`, parsing, display, descriptions, tests |
| Single-role vs multi-role change | CLI shape, role discovery, schedule parsing, run dispatch |
| Issue-driven toggle change | `run` argument handling, workflow routing, `single_role` execution path |
| Branching/merge policy change | Static `allowed_prefixes` array in automerge workflow template, layer contracts `branch_prefix` |
| Artifact contract change | Scaffold schemas, doctor schema checks, mock artifact generators |

## Adding a Layer: Change Order

1. Domain model and parsing
- Add enum variant and all layer metadata in `src/domain/workspace/layer.rs`.
- Keep classification (`is_single_role`, `is_issue_driven`) coherent.

2. Scaffold and contracts
- Add `.jules/roles/<layer>/contracts.yml`, `prompt_assembly.yml`, and schemas under `src/assets/scaffold/.jules/roles/<layer>/`.
- Keep assets file-based; do not move schema content into Rust string constants.

3. CLI and runtime dispatch
- Extend `run` command parsing in `src/app/cli/run.rs`.
- Extend workflow layer parsing and command handling in `src/app/cli/workflow.rs`.
- Wire execution in `src/app/commands/run/mod.rs` and `src/app/commands/workflow/run/layer_executor.rs`.

4. Schedule and role selection (if multi-role)
- Extend `WorkstreamSchedule` in `src/domain/configuration/schedule.rs`.
- Keep absent/empty schedule semantics explicit and deterministic.

5. Doctor and workspace contracts
- Extend structure/schema/semantic checks in `src/app/commands/doctor/*.rs`.
- Ensure doctor validates new layer contracts and new workstream artifacts.

6. Workflow templates
- Integrate layer phase into `src/assets/workflows/.github/workflows/jules-workflows.yml.j2`.
- Confirm entry-point and wait gating remain coherent.

7. Mock and cleanup
- Add mock execution module in `src/app/commands/run/mock/`.
- Add cleanup scope for new mock artifacts in `src/app/commands/workflow/cleanup/mock.rs`.

8. Tests and docs
- Update integration tests that assert layer count, structure, workflow text, and mock behavior.
- Update docs under `docs/` and other relevant READMEs as needed.

## Removing a Layer: Change Order

1. Remove runtime entry points and dispatch branches
- Remove CLI exposure and `match` branches in run/workflow commands.

2. Remove scaffold and template assets
- Remove obsolete role directory under `src/assets/scaffold/.jules/roles/`.
- Remove related workstream artifacts only when no other layer consumes them.

3. Remove workflow phases
- Remove corresponding jobs and dependencies from workflow templates.

4. Remove mock generation and recovery hooks
- Remove per-layer mock module and cleanup selectors.

5. Remove validation and tests
- Delete doctor checks tied only to removed artifacts.
- Delete/update tests that expect removed layer behavior.

## Workflow Maintenance Invariants
- Workflow kit source of truth is `src/assets/workflows/.github/`; generated `.github/` files are installation outputs.
- Auto-merge branch matching is driven by a static `allowed_prefixes` array in the automerge workflow template, not by runtime contract scanning.
- Adding or removing a layer requires updating the `allowed_prefixes` array in `jules-automerge.yml.j2` and regenerating workflows.
- `.jules/`-only scope remains the automerge safety boundary.
- Control-plane files live under `.jlo/` on the control branch; `.jules/` is materialized by workflow bootstrap.
- `install-jlo` reads the version pin from `JLO_TARGET_BRANCH` `.jlo/.jlo-version`, not from `origin/JULES_WORKER_BRANCH`.
- Projection from `.jlo/` to `.jules/` never overwrites agent-generated exchange artifacts.

## Mock Maintenance Invariants
- Mock mode is a first-class execution path, not a test stub.
- Each layer with workflow participation has explicit mock semantics.
- Recovery cleanup handles partial failures and removes only mock-scoped artifacts.

## Quick Impact Check Before Merge
- Layer enum and parsing updated.
- Scaffold assets and contracts added/removed consistently.
- Run/workflow dispatch updated.
- Schedule model updated if layer is multi-role.
- Doctor checks updated for new/removed artifacts.
- Workflow templates updated.
- Mock behavior and cleanup updated.
- Tests updated (`cli_flow`, `cli_commands`, `workflow_kit`, `mock_mode`).
