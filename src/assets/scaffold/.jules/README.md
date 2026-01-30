# .jules/

The `.jules/` directory is a structured workspace for scheduled agents and human execution.
It captures **observations as events** and **actionable work as issues** within workstreams.

This file is human-oriented. Agents must read `.jules/JULES.md` for the formal contract.

## Scope (What this README defines)

- `.jules/` defines **artifacts and contracts** (events/issues, role state, schemas).
- **Execution + git + PR operations are out of scope here.**
  When you are reading this inside the VM, you already have the execution environment;
  follow the role contract and produce the required artifacts/changes.
- `jlo` is **scaffold + prompt/config management only**.
  It does **not** run agents, orchestrate schedules, create PRs, or manage merges.

## Document Hierarchy

| Document | Audience | Contains |
|----------|----------|----------|
| `AGENTS.md` | All LLM tools | Repository conventions (exposed to Codex, Claude, etc.) |
| `.jules/JULES.md` | Jules agents | Jules-specific contracts and schemas |
| `.jules/README.md` | Humans | This guide |

**Rule**: Jules-internal details stay in `.jules/`. `AGENTS.md` remains tool-agnostic.

## Role Flow

```
Observer -> Decider -> [Planner] -> Implementer
(events)    (issues)   (expand)     (code changes)
```

| Role Type | Role(s) | Transformation |
|-----------|---------|----------------|
| Observer | directories under `.jules/roles/observers/` | Source -> Events (domain-specialized observations) |
| Decider | directories under `.jules/roles/deciders/` | Events -> Issues (validation + consolidation) |
| Planner | directories under `.jules/roles/planners/` | Issues -> Expanded Issues (deep analysis, optional) |
| Implementer | directories under `.jules/roles/implementers/` | Issues -> Code changes |

**Execution**: All roles are invoked by GitHub Actions via `jules-invoke`.

**Configuration Language**: All YAML files are written in English for optimal LLM processing.

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Observer | `jules` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `jules` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `jules` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `main` | `jules-implementer-*` | ❌ (human review) |

Observers, Deciders, and Planners modify only `.jules/` and auto-merge after CI passes.
Implementers modify source code and require human review.

## Directory Structure

```
.jules/
+-- README.md           # This file (jlo-managed)
+-- JULES.md            # Agent contract (jlo-managed)
+-- .jlo-version        # Version marker (jlo-managed)
|
+-- workstreams/        # Workstream containers
|   +-- <workstream>/   # e.g. generic/
|       +-- events/     # Raw observations
|       |   +-- *.yml
|       +-- issues/     # Consolidated problems
|           +-- index.md
|           +-- high/
|           +-- medium/
|           +-- low/
|
+-- roles/              # Role definitions (global)
    +-- observers/      # Multi-role layer
    |   +-- contracts.yml    # Shared observer contract
    |   +-- event.yml        # Event template
    |   +-- <role>/          # Role subdirectory
    |       +-- prompt.yml   # Static: run prompt (includes workstream)
    |       +-- role.yml     # Dynamic: evolving focus
    |       +-- notes/       # Declarative state
    |       +-- feedbacks/   # Decider feedback
    |
    +-- deciders/       # Multi-role layer
    |   +-- contracts.yml    # Shared decider contract
    |   +-- issue.yml        # Issue template
    |   +-- feedback.yml     # Feedback template
    |   +-- <role>/          # Role subdirectory
    |       +-- prompt.yml   # Includes workstream
    |
    +-- planners/       # Single-role layer (issue-driven)
    |   +-- contracts.yml    # Shared planner contract
    |   +-- prompt.yml       # Direct prompt (no subdirectory)
    |
    +-- implementers/   # Single-role layer (issue-driven)
        +-- contracts.yml    # Shared implementer contract
        +-- prompt.yml       # Direct prompt (no subdirectory)
```

## Layer Architecture

| Layer | Type | Invocation |
|-------|------|------------|
| Observers | Multi-role | `jlo run observers` |
| Deciders | Multi-role | `jlo run deciders` |
| Planners | Single-role | `jlo run planners --issue <path>` |
| Implementers | Single-role | `jlo run implementers --issue <path>` |

**Multi-role layers** (Observers, Deciders): Support multiple configurable roles listed in `config.toml`. Each role has its own subdirectory with `prompt.yml`. Custom roles can be created with `jlo template`.

**Single-role layers** (Planners, Implementers): Have a fixed role with `prompt.yml` directly in the layer directory. They are issue-driven and require the `--issue` flag. Template creation is not supported.

## Workstreams

Workstreams isolate events and issues so that decider rules do not mix across unrelated operational areas.

- A workstream may run observers only (no decider), leaving events for human review.
- `roles/` remains global (not nested per workstream).
- Observers and deciders declare their destination workstream in `prompt.yml`.
- If the workstream directory is missing, execution fails fast.

The default scaffold creates a `generic` workstream.

## Configuration Files

### contracts.yml
Layer-level shared constraints and workflows. All roles in the layer reference this file.

### prompt.yml
Execution parameters and references to contracts.yml. Includes `workstream:` field for observers and deciders.

### role.yml
Specialized focus that evolves through feedback loop. Only observers have this (stateful layer).

### Templates (*.yml)
Copyable templates (event.yml, issue.yml, feedback.yml) defining the structure of artifacts.
Agents `cp` these files and fill them out.

## Workflow

### 1. Observer Agents (Scheduled)

Each observer:
1. Reads contracts.yml (layer behavior)
2. Reads role.yml (specialized focus)
3. Reads feedbacks/, abstracts patterns, updates role.yml
4. Reads notes/ for current state
5. **Reads .jules/workstreams/<workstream>/issues/index.md to check for open issues**
6. Updates notes/ declaratively
7. **Skips observations already covered by open issues (deduplication)**
8. Writes workstreams/<workstream>/events/*.yml when observations warrant
9. Publishes changes as a PR (branch naming follows the convention below)

**Stateful**: Maintains `notes/` and receives feedback via `feedbacks/`.

### 2. Decider Agent (Scheduled)

Triage agent:
1. Reads contracts.yml (layer behavior)
2. Reads all workstreams/<workstream>/events/*.yml
3. **Reads .jules/workstreams/<workstream>/issues/index.md and existing issues to identify merge candidates**
4. Validates observations (do they exist in codebase?)
5. Merges related events sharing root cause
6. **Merges events into existing issues when related (updates content)**
7. Creates new issues for genuinely new problems (using fingerprint as filename, placing in priority folder)
8. **Updates .jules/workstreams/<workstream>/issues/index.md**
9. **When deep analysis is needed, provides clear rationale in deep_analysis_reason**
10. Writes feedback for recurring rejections
11. Deletes processed events

**Decider answers**: "Is this real? Should these events merge into one issue?"

### 3. Planner Agent (On-Demand)

Specifier agent (runs only for `requires_deep_analysis: true`):
1. Reads contracts.yml (layer behavior)
2. Reads target issue from workstreams/<workstream>/issues/<priority>/
3. **Reviews deep_analysis_reason to understand scope**
4. Analyzes full system impact and dependency tree
5. Expands issue with detailed analysis (affected_areas, constraints, risks)
6. Sets requires_deep_analysis to false
7. **Preserves and expands the original rationale with findings**
8. Overwrites the issue file

**Planner answers**: "What is the full scope of this issue?"

### 4. Implementation (Via Local Issue)

Implementation is invoked manually via `workflow_dispatch` with a local issue file path.

```bash
# Example: Run implementer with a specific issue
jlo run implementers --issue .jules/workstreams/generic/issues/medium/auth_inconsistency.yml
```

The implementer reads the issue content (embedded in prompt) and produces code changes.
The issue file must exist; missing files fail fast before agent execution.

**Issue Lifecycle**:
1. User selects an issue file from `.jules/workstreams/<workstream>/issues/<priority>/` on the `jules` branch.
2. Workflow validates the file exists and passes content to the implementer.
3. After successful dispatch, the issue file is automatically deleted from the `jules` branch.
4. The implementer works on `main` branch and creates a PR for human review.
5. When the PR is merged, `sync-jules.yml` syncs `main` back to `jules`.

## Feedback Loop

```
Observer creates events in workstreams/<workstream>/events/
       |
       v
Decider validates, may reject or merge
       |
       v (rejection)
Decider writes feedbacks/{date}_{desc}.yml
       |
       v
Observer reads feedbacks/, updates role.yml
       |
       v
Observer avoids similar observations
```

Feedback files are preserved for audit (never deleted).

## Issue Lifecycle

- Issues are organized by priority (`low`, `medium`, `high`) and tracked in `index.md`.
- Open issues suppress duplicate observations from observers.
- Issue filenames use stable fingerprints (e.g. `auth_inconsistency.yml`).
- Related events are merged into existing issues, not duplicated.

## Branch Naming Convention

All agents must create branches using this format:

```
jules-observer-<id>
jules-decider-<id>
jules-planner-<id>
jules-implementer-<fingerprint>-<short_description>
```

## Testing and Validation

The mock pipeline workflow generates synthetic workstream artifacts and exercises the observer → decider → planner transitions without calling external APIs.

## Pause/Resume

Set the repository variable `JULES_PAUSED=true` to skip scheduled runs.
The default behavior is active; paused behavior is explicit and visible.
