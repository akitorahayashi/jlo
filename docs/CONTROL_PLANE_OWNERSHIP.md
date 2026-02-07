# Control-Plane Ownership Model

## Branch Topology

| Branch | Purpose | Editable by |
|--------|---------|-------------|
| Control branch (e.g. `main`, `development`) | Hosts `.jlo/` intent overlay and `.github/` workflow kit | User (via `jlo init`, `jlo update`, manual edits) |
| `jules` | Hosts materialized `.jules/` runtime state and agent exchange artifacts | Workflow bootstrap only (never user-edited directly) |

Users never checkout or edit the `jules` branch directly. All configuration is performed on the control branch under `.jlo/`.

## Directory Ownership

### `.jlo/` — Intent Overlay (control branch)

`.jlo/` is a minimal directory containing only the version pin and durable user intent inputs. Managed framework assets (contracts, schemas, prompts, global documents) are **not** stored in `.jlo/`; they are materialized by workflow bootstrap from the embedded scaffold for the pinned version.

| Path | Owner | Description |
|------|-------|-------------|
| `.jlo/.jlo-version` | jlo | Pinned jlo binary version. Written by `init`, advanced by `update`. |
| `.jlo/config.toml` | User | Workspace configuration. Created by `init`; never overwritten. |
| `.jlo/roles/<layer>/roles/<role>/role.yml` | User | Role-specific customizations. Created by `template`; never overwritten. |
| `.jlo/workstreams/<ws>/scheduled.toml` | User | Workstream schedule and role roster. Created by `template`; never overwritten. |
| `.jlo/setup/tools.yml` | User | Tool selection. Created by `init`; never overwritten. |

### `.jules/` — Runtime Data Plane (jules branch)

`.jules/` is the complete runtime workspace materialized by workflow bootstrap. It combines scaffold framework assets (for the pinned version) with user intent overlays from `.jlo/`.

| Path | Owner | Written by | Description |
|------|-------|------------|-------------|
| `.jules/.jlo-version` | Bootstrap | Workflow bootstrap | Copied from `.jlo/.jlo-version` |
| `.jules/JULES.md` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/README.md` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/config.toml` | Bootstrap | Workflow bootstrap | Materialized from `.jlo/config.toml` |
| `.jules/github-labels.json` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/roles/<layer>/contracts.yml` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/roles/<layer>/prompt.yml` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/roles/<layer>/prompt_assembly.yml` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/roles/<layer>/schemas/*.yml` | Bootstrap | Workflow bootstrap | Materialized from embedded scaffold |
| `.jules/roles/<layer>/roles/<role>/role.yml` | Bootstrap | Workflow bootstrap | Materialized from `.jlo/` user overlay |
| `.jules/workstreams/<ws>/scheduled.toml` | Bootstrap | Workflow bootstrap | Materialized from `.jlo/` user overlay |
| `.jules/workstreams/<ws>/exchange/events/**` | Agent | Agent execution | Observer outputs, decider inputs |
| `.jules/workstreams/<ws>/exchange/issues/**` | Agent | Agent execution | Decider outputs, planner/implementer inputs |
| `.jules/workstreams/<ws>/exchange/innovators/**` | Agent | Agent execution | Innovator artifacts |
| `.jules/changes/latest.yml` | Agent | Narrator execution | Changes summary |
| `.jules/setup/**` | Bootstrap | Workflow bootstrap | Materialized from `.jlo/setup/` + scaffold |
| `.jules/.managed-defaults.yml` | Bootstrap | Workflow bootstrap | Materialized manifest |

### `.github/` — Workflow Kit (control branch)

| Path | Owner | Installed by | Description |
|------|-------|--------------|-------------|
| `.github/workflows/jules-*.yml` | jlo | `jlo init` | Workflow definitions |
| `.github/actions/install-jlo/**` | jlo | `jlo init` | jlo installer action |
| `.github/actions/configure-git/**` | jlo | `jlo init` | Git configuration action |
| `.github/actions/run-implementer/**` | jlo | `jlo init` | Implementer execution action |

## Classification Rules

| Classification | Definition | Lives in |
|----------------|------------|----------|
| **Version pin** | The `.jlo-version` file that locks the jlo binary version. Advanced by `jlo update`. | `.jlo/` |
| **User intent** | Configuration, schedules, role customizations, tool selections. Created once by `init` or `template`; owned by the user thereafter. | `.jlo/` |
| **Managed framework** | Contracts, schemas, prompts, global documents. Content is determined entirely by the jlo version. | Embedded scaffold → materialized to `.jules/` by bootstrap |
| **Agent-generated** | Runtime artifacts written by agent execution. Never touched by bootstrap, update, or projection. | `.jules/` exchange paths |

## Materialization Boundary

Workflow bootstrap is the sole authority for producing `.jules/` on the `jules` branch.

### Bootstrap Algorithm

1. Read `.jlo/.jlo-version` from the control branch.
2. Load embedded scaffold assets for that version.
3. Checkout `jules` branch (create from orphan if absent).
4. Write all managed framework files from embedded scaffold to `.jules/`.
5. Overlay user intent files from `.jlo/` (config, schedules, role customizations) into `.jules/`.
6. Ensure structural directories exist (layer dirs, workstream exchange dirs with `.gitkeep`).
7. Never delete or modify paths under `.jules/workstreams/*/exchange/` or `.jules/changes/`.
8. Commit changes (if any) to `jules` with a deterministic message.

### Idempotency

Running bootstrap twice with the same `.jlo/` inputs and jlo version produces no new commits on `jules`. The algorithm is compare-then-write: files are only written when content differs.

## Update Semantics

`jlo update` is a control-plane version-pin advancement operation, not runtime file reconciliation.

| Action | Description |
|--------|-------------|
| Advance `.jlo/.jlo-version` | Write the current binary version to the version pin. |
| Reconcile user intent files | Create missing user-owned files from scaffold defaults without overwriting existing ones. |
| **Not in scope** | Patching managed framework files (that is bootstrap's responsibility on `jules`). |
| **Not in scope** | Reading or writing `.jules/` or any runtime artifacts. |
| **Not in scope** | Reading or writing `.jules/workstreams/*/exchange/` (agent-generated). |

Runtime managed assets are expanded from the scaffold for the pinned version during the next workflow bootstrap run.

## Version Pin Flow

1. `jlo init` writes current binary version to `.jlo/.jlo-version` on the control branch.
2. `jlo update` advances `.jlo/.jlo-version` on the control branch.
3. Workflow `install-jlo` action reads `.jlo/.jlo-version` from the control branch (not `origin/jules`).
4. Workflow bootstrap reads `.jlo/.jlo-version`, loads corresponding scaffold, and materializes `.jules/` on `jules`.

## Failure Policy

| Condition | Behavior |
|-----------|----------|
| `.jlo/` missing on control branch | Hard failure. Bootstrap aborts with explicit error. |
| `.jlo/.jlo-version` missing or empty | Hard failure. Version pin is required. |
| `jules` branch does not exist | Bootstrap creates it as an orphan branch with initial materialized content. |
| `.jules/` missing on existing `jules` | Bootstrap performs full materialization. |
| Git commit failure during bootstrap | Hard failure. Workflow aborts with details. No silent retry. |
| `jlo init` on `jules` branch | Rejected. Init creates `.jlo/` which belongs on the control branch. |
| Exchange artifacts during projection | Skipped unconditionally. Agent-generated files are never touched. |
