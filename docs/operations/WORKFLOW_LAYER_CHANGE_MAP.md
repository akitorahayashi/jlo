# Workflow Layer Change Map

This document defines the current repository touch points for layer-level changes in `jlo`.

A layer-level change means one of the following:
- Adding a new layer.
- Removing an existing layer.
- Changing a layer type (single-role, multi-role, issue-driven).
- Changing layer artifacts used by workflow execution and validation.

## Authoritative Sources
- Layer model: `src/domain/layers/mod.rs`
- Control-plane ownership: `docs/architecture/CONTROL_PLANE_OWNERSHIP.md`
- CLI entry points: `src/app/cli/run.rs`, `src/app/cli/workflow.rs`
- Runtime dispatch: `src/app/commands/run/mod.rs`, `src/app/commands/workflow/run/layer/mod.rs`
- Schedule model: `src/domain/schedule/model.rs`
- Scaffold assets: `src/assets/scaffold/jules/**`
- Workflow templates: `src/assets/github/workflows/*.j2`
- Mock execution: `src/app/commands/run/mock/**`
- Mock recovery cleanup: `src/app/commands/workflow/exchange/clean/mock.rs`

## Layer Change Surface

| Surface | Current coupling | Main files |
|---|---|---|
| Layer identity and parsing | `Layer::ALL`, `dir_name`, `from_dir_name`, type flags are hard-coded | `src/domain/layers/mod.rs` |
| CLI command shape | `run` subcommands are explicit per layer | `src/app/cli/run.rs` |
| Workflow command parsing | `workflow run <layer>` relies on `Layer::from_dir_name` | `src/app/cli/workflow.rs` |
| Runtime execution branch | Main `run` path and workflow layer dispatcher match on layer enum | `src/app/commands/run/mod.rs`, `src/app/commands/workflow/run/layer/mod.rs` |
| Multi-role scheduling | `.jlo/config.toml` models observer and innovator roles explicitly | `src/domain/schedule/model.rs`, `src/domain/config/run.rs` |
| Prompt and contracts | Layer assets are file-based and embedded with `include_dir` | `src/assets/scaffold/jules/layers/<layer>/**` |
| Doctor validation | Structural/schema/semantic checks iterate layers and exchange data contracts | `src/app/commands/doctor/*.rs` |
| Requirement routing | Exchange inspect provides requirement counts for planner/implementer gating | `src/app/commands/workflow/exchange/inspect.rs`, `src/app/commands/workflow/run/*.rs` |
| Workflow orchestration | Layer sequence is defined in workflow templates; integrator has a dedicated manual-dispatch workflow | `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`, `src/assets/github/workflows/jules-integrator.yml.j2` |
| Auto-merge qualification | Branch prefix and scope policy gates are evaluated in `jlo workflow gh process pr automerge` | `src/app/commands/workflow/gh/pr/events/enable_automerge.rs` |
| Mock behavior | Per-layer mock behavior is implemented in dedicated modules | `src/app/commands/run/mock/*.rs` |
| Failure recovery | Mock residue cleanup scope is explicit and code-defined | `src/app/commands/workflow/exchange/clean/mock.rs` |
| Tests | Integration tests assert layer structure, workflow text, and mock behavior | `tests/workflow.rs`, `tests/cli.rs`, `tests/mock.rs`, `tests/doctor.rs` |

## Layer Types and Downstream Effects

| Layer trait change | Downstream code that changes |
|---|---|
| Add/remove layer in enum | `Layer::ALL`, parsing, display, descriptions, tests |
| Single-role vs multi-role change | CLI shape, role discovery, schedule parsing, run dispatch |
| Issue-driven toggle change | `run` argument handling, workflow routing, `single_role` execution path |
| Branching/merge policy change | `automerge` gate policy in command code, layer contracts `branch_prefix` |
| Artifact contract change | Scaffold schemas, doctor schema checks, mock artifact generators |

## Adding a Layer: Change Order

1. Domain model and parsing
- Add enum variant and all layer metadata in `src/domain/layers/mod.rs`.
- Keep classification (`is_single_role`, `is_issue_driven`) coherent.

2. Scaffold and contracts
- Add `.jules/layers/<layer>/contracts.yml`, `<layer>_prompt.j2`, and schemas under `src/assets/scaffold/jules/layers/<layer>/`.
- Keep assets file-based; do not move schema content into Rust string constants.

3. CLI and runtime dispatch
- Extend `run` command parsing in `src/app/cli/run.rs`.
- Extend workflow layer parsing and command handling in `src/app/cli/workflow.rs`.
- Wire execution in `src/app/commands/run/mod.rs` and `src/app/commands/workflow/run/layer/mod.rs`.

4. Schedule and role selection (if multi-role)
- Extend `Schedule` in `src/domain/schedule/model.rs`.
- Keep absent/empty schedule semantics explicit and deterministic.

5. Doctor and runtime contracts
- Extend structure/schema/semantic checks in `src/app/commands/doctor/*.rs`.
- Ensure doctor validates new layer contracts and new exchange artifacts.

6. Workflow templates
- Integrate layer phase into `src/assets/github/workflows/jules-scheduled-workflows.yml.j2`.
- Confirm entry-point and wait gating remain coherent.

7. Mock and cleanup
- Add mock execution module in `src/app/commands/run/mock/`.
- Add cleanup scope for new mock artifacts in `src/app/commands/workflow/exchange/clean/mock.rs`.

8. Tests and docs
- Update integration tests that assert layer count, structure, workflow text, and mock behavior.
- Update docs under `docs/` and other relevant READMEs as needed.

## Removing a Layer: Change Order

1. Remove runtime entry points and dispatch branches
- Remove CLI exposure and `match` branches in run/workflow commands.

2. Remove scaffold and template assets
- Remove obsolete role directory under `src/assets/scaffold/jules/layers/`.
- Remove related exchange artifacts only when no other layer consumes them.

3. Remove workflow phases
- Remove corresponding jobs and dependencies from workflow templates.

4. Remove mock generation and recovery hooks
- Remove per-layer mock module and cleanup selectors.

5. Remove validation and tests
- Delete doctor checks tied only to removed artifacts.
- Delete/update tests that expect removed layer behavior.

## Workflow Maintenance Invariants
- Workflow kit source of truth is `src/assets/github/`; generated `.github/` files are installation outputs.
- Auto-merge policy gates (branch prefix, `.jules/`-only scope, draft, already-enabled) are evaluated in `src/app/commands/workflow/gh/pr/events/enable_automerge.rs`. The workflow template delegates to `jlo workflow gh pr process automerge`.
- Adding or removing a layer requires updating the `ALLOWED_PREFIXES` constant in `enable_automerge.rs` and regenerating workflows.
- `.jules/`-only scope remains the automerge safety boundary.
- Control-plane files live under `.jlo/` on the control branch; `.jules/` is materialized by workflow bootstrap.
- `install-jlo` reads the version pin from `JLO_TARGET_BRANCH` `.jlo/.jlo-version`, not from `origin/JULES_WORKER_BRANCH`.
- Bootstrap materialization of `.jules/` never overwrites agent-generated exchange artifacts.

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
- Tests updated (`cli`, `workflow`, `doctor`, `mock`).
