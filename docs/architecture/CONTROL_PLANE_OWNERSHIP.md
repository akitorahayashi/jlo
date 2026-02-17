# Control-Plane Ownership Model

## Branch Topology

| Branch | Purpose | Editable by |
|--------|---------|-------------|
| `JLO_TARGET_BRANCH` | Hosts `.jlo/` intent overlay and `.github/` workflow kit | User (via `jlo init`, `jlo update`, manual edits) |
| `JULES_WORKER_BRANCH` | Hosts materialized `.jules/` runtime state and agent exchange artifacts | Workflow bootstrap only (never user-edited directly) |

Users never checkout or edit the `JULES_WORKER_BRANCH` branch directly. All configuration is performed on `JLO_TARGET_BRANCH` under `.jlo/`.
`jlo init` installs the control-plane and workflow kit only; `.jules/` is created by workflow bootstrap on `JULES_WORKER_BRANCH`.

## Directory Ownership

### `.jlo/` — Intent Overlay (control branch)

`.jlo/` is a minimal directory containing only the version pin and durable user intent inputs. Managed framework assets (contracts, schemas, prompts, global documents) are **not** stored in `.jlo/`; they are materialized by workflow bootstrap from the embedded scaffold for the pinned version. Built-in role definitions are embedded in the jlo binary under `src/assets/roles/` and resolved at runtime when a matching custom role file is absent.

| Path | Owner | Description |
|------|-------|-------------|
| `.jlo/.jlo-version` | jlo | Pinned jlo binary version. Written by `init`, advanced by `update`. |
| `.jlo/config.toml` | User | Workspace configuration and schedule roster (`[observers].roles`, `[innovators].roles`). Created by `init`; never overwritten. |
| `.jlo/roles/<layer>/<role>/role.yml` | User | Role-specific customizations. Created by `create`; custom roles override built-ins at runtime. |
| `.jlo/setup/tools.yml` | User | Tool selection. Created by `init`; never overwritten. |

### `.jules/` — Runtime Data Plane (worker branch)

`.jules/` is the complete runtime workspace materialized by workflow bootstrap from embedded scaffold assets for the pinned version. Control intent stays in `.jlo/` and is read directly when needed.

| Path | Owner | Written by | Description |
|------|-------|------------|-------------|
| `.jules/.jlo-version` | Bootstrap | Workflow bootstrap | Copied from `.jlo/.jlo-version` |
| `.jules/JULES.md` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/README.md` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/exchange/events/<state>/**` | Agent | Agent execution | Observer outputs, decider inputs |
| `.jules/exchange/requirements/**` | Agent | Agent execution | Decider outputs, planner/implementer inputs |
| `.jules/exchange/proposals/*.yml` | Agent | Agent execution | Innovator proposals pending publication |
| `.jules/workstations/<role>/**` | Agent | Agent execution | Role perspectives (memory) |
| `.jules/exchange/changes.yml` | Agent | Narrator execution | Changes summary |
| `.jules/setup/**` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |

### `.github/` — Workflow Kit (control branch)

| Path | Owner | Installed by | Description |
|------|-------|--------------|-------------|
| `.github/workflows/jules-*.yml` | jlo | `jlo init` | Workflow definitions |
| `.github/actions/install-jlo/**` | jlo | `jlo init` | jlo installer action |
| `.github/actions/configure-git/**` | jlo | `jlo init` | Git configuration action |

## Classification Rules

| Classification | Definition | Lives in |
|----------------|------------|----------|
| **Version pin** | The `.jlo-version` file that locks the jlo binary version. Advanced by `jlo update`. | `.jlo/` |
| **User intent** | Configuration, schedule rosters, role customizations, tool selections. Created once by `init` or `template`; owned by the user thereafter. | `.jlo/` |
| **Managed framework** | Contracts, schemas, prompts, global documents. Content is determined entirely by the jlo version. | Embedded scaffold → materialized to `.jules/` by bootstrap |
| **Agent-generated** | Runtime artifacts written by agent execution. Never touched by bootstrap, update, or projection. | `.jules/` exchange paths |

## Materialization Boundary

Workflow bootstrap is the sole authority for producing `.jules/` on `JULES_WORKER_BRANCH`.

### Bootstrap Algorithm

1. Verify `.jlo/` and `.jlo/.jlo-version` exist (hard preconditions).
2. Load embedded scaffold assets for the pinned version.
3. Checkout `JULES_WORKER_BRANCH` (create from `JLO_TARGET_BRANCH` history if absent).
4. Write all managed framework files from embedded scaffold to `.jules/`.
5. Do not project user intent files from `.jlo/` into `.jules/`; agents read control intent directly.
6. Ensure workstation perspectives for scheduled observer/innovator roles.
7. Prune workstation directories for unscheduled roles.
8. Do not prune unmanaged runtime artifacts.
9. Write the runtime version file.
10. Commit changes (if any) to `JULES_WORKER_BRANCH` with a deterministic message.

### Idempotency

Running bootstrap twice with the same `.jlo/` inputs and jlo version produces no new commits on `JULES_WORKER_BRANCH`. The algorithm is compare-then-write: files are only written when content differs.

## Update Semantics

`jlo update` is a control-plane maintenance operation that advances the version pin and refreshes the workflow kit.

| Action | Description |
|--------|-------------|
| Advance `.jlo/.jlo-version` | Write the current binary version to the version pin. |
| Reconcile control-plane skeleton | Create missing control-plane files from scaffold defaults without overwriting existing ones. |
| Refresh workflow kit | Reinstall `.github/` workflows using `.jlo/config.toml` `workflow.runner_mode`. |
| **Not in scope** | Patching managed framework files (that is bootstrap's responsibility on `JULES_WORKER_BRANCH`). |
| **Not in scope** | Reading or writing `.jules/` or any runtime artifacts. |
| **Not in scope** | Reading or writing `.jules/exchange/` (agent-generated). |

Runtime managed assets are expanded from the scaffold for the pinned version during the next workflow bootstrap run.

## Version Pin Flow

1. `jlo init` writes current binary version to `.jlo/.jlo-version` on the control branch.
2. `jlo update` advances `.jlo/.jlo-version` on the control branch.
3. Workflow `install-jlo` action reads `.jlo/.jlo-version` from `JLO_TARGET_BRANCH`.
4. Workflow bootstrap reads `.jlo/.jlo-version`, loads corresponding scaffold, and materializes `.jules/` on `JULES_WORKER_BRANCH`.

## Failure Policy

| Condition | Behavior |
|-----------|----------|
| `.jlo/` missing on control branch | Hard failure. Bootstrap aborts with explicit error. |
| `.jlo/.jlo-version` missing or empty | Hard failure. Version pin is required. |
| `JULES_WORKER_BRANCH` does not exist | Bootstrap creates it from `JLO_TARGET_BRANCH` history with initial materialized content. |
| `.jules/` missing on existing `JULES_WORKER_BRANCH` | Bootstrap performs full materialization. |
| Git commit failure during bootstrap | Hard failure. Workflow aborts with details. No silent retry. |
| `jlo init` on `JULES_WORKER_BRANCH` | Rejected. Init creates `.jlo/` which belongs on `JLO_TARGET_BRANCH`. |
| Exchange artifacts during projection | Skipped unconditionally. Agent-generated files are never touched. |
