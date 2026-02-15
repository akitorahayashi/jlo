# .jules/

The `.jules/` directory is a structured runtime repository for scheduled agents and human execution.
It captures **observations as events** and **actionable work as requirements** within the exchange.

This file is human-oriented. Agents must read `.jules/JULES.md` for the formal contract.

## Scope (What this README defines)

- `.jules/` defines **artifacts and contracts** (events/requirements, role state, schemas).
- **Execution + git + PR operations are out of scope here.**
  When you are reading this inside the VM, you already have the execution environment;
  follow the role contract and produce the required artifacts/changes.
- `jlo` is **scaffold + prompt/config management only**.
  It does **not** run agents, orchestrate schedules, create PRs, or manage merges.

## Document Hierarchy

| Document | Audience | Contains |
|----------|----------|----------|
| `.jules/JULES.md` | Jules agents | Jules-specific contracts and schemas |
| `.jules/README.md` | Humans | This guide |

**Rule**: Jules-internal details stay in `.jules/`.

## Role Flow

```
Narrator -> Observer -> Decider -> [Planner] -> Implementer
(changes)   (events)    (requirements) (expand)  (code changes)
```

| Role Type | Role(s) | Transformation |
|-----------|---------|----------------|
| Narrator | `.jules/layers/narrator/` | Git history -> Changes summary |
| Observer | directories under `.jlo/roles/observers/` | Source -> Events (domain-specialized observations) |
| Decider | `.jules/layers/decider/` | Events -> Requirements (validation + consolidation) |
| Planner | (Single-role; no `.jlo/` role definitions) | Requirements -> Expanded Requirements (deep analysis, optional) |
| Implementer | (Single-role; no `.jlo/` role definitions) | Requirements -> Code changes |

**Execution**: Roles are invoked by GitHub Actions using `jlo run` and workflow dispatch workflows.

**Configuration Language**: All YAML files are written in English for optimal LLM processing.

## Branch Strategy

| Agent Type | Starting Branch | Output Branch | Auto-merge |
|------------|-----------------|---------------|------------|
| Narrator | `Worker Branch` | `jules-narrator-*` | ✅ (if `.jules/` only) |
| Observer | `Worker Branch` | `jules-observer-*` | ✅ (if `.jules/` only) |
| Decider | `Worker Branch` | `jules-decider-*` | ✅ (if `.jules/` only) |
| Planner | `Worker Branch` | `jules-planner-*` | ✅ (if `.jules/` only) |
| Implementer | `Target Branch` | `jules-implementer-*` | ❌ (human review) |
| Integrator | `Target Branch` | `jules-integrator-*` | ❌ (human review) |

Narrator, Observers, Decider, and Planner modify only `.jules/` and auto-merge after CI passes.
Implementer modifies source code and requires human review.

## Directory Structure

```
.jules/
├── exchange/
│   ├── events/
│   ├── proposals/
│   └── requirements/
├── layers/
│   ├── decider/
│   ├── implementer/
│   ├── innovators/
│   ├── integrator/
│   ├── narrator/
│   ├── observers/
│   └── planner/
└── workstations/
```

## Layer Architecture

| Layer | Type | Invocation |
|-------|------|------------|
| Narrator | Single-role | `jlo run narrator` |
| Observers | Multi-role | `jlo run observers --role <role>` |
| Decider | Single-role | `jlo run decider` |
| Planner | Single-role | `jlo run planner <path>` |
| Implementer | Single-role | `jlo run implementer <path>` |
| Integrator | Single-role | `jlo run integrator` |

**Narrator**: Produces `.jules/exchange/changes.yml` summarizing recent codebase changes. Runs first, before observers. Observers treat this as a secondary hint, not as a scope driver.

**Multi-role layers** (Observers, Innovators): Roles are scheduled via `.jlo/config.toml` (`[observers].roles`, `[innovators].roles`). Custom roles can be authored under `.jlo/roles/` with `jlo create <layer> <name>`, while built-ins are registered in config with `jlo add <layer> <role>` and resolved from embedded assets when no custom role file exists.

**Single-role layers** (Decider, Planner, Implementer): Have a fixed role with `contracts.yml` directly in the layer directory. Planner and Implementer are requirement-driven and require a requirement file path argument. Template creation is not supported.

**Integrator**: A manual, on-demand layer that merges all remote `jules-implementer-*` branches into a single integration branch. PR discussions are retrieved live via `gh` during execution. Not part of the scheduled orchestration chain.

**Innovators**: Task-driven execution (`--task <task_name>`). Current flow uses `tasks/create_three_proposals.yml` to emit three proposals in one run and update workstation memory. Universal constraints are in `contracts.yml`.

## Exchange
 
 The exchange is a flat directory structure for events and requirements.
 
 - Events and requirements are global to the repository.
 - `roles/` remains global.
 - Observers and Decider operate on the single `exchange/` directory.
 - Innovators submit proposals to the `exchange/proposals/` directory.
 - Event state directories are defined by the scaffold templates.

## Configuration Files

### contracts.yml
Layer-level shared constraints. All roles in the layer reference this file.

### <layer>_prompt.j2
Defines a prompt template that assembles required and optional includes into a single prompt sent to the agent.

### role.yml
Specialized focus for observers and innovators. Continuity lives in the workstation perspective file.

### Templates (*.yml)
Copyable templates (changes.yml, event.yml, requirements.yml) defining the structure of artifacts.
Agents `cp` these files and fill them out.

## Workflow

### 0. Narrator Agent (Scheduled, runs first)

Narrator summarizes codebase changes as secondary hint context for observer triage:
1. Reads `.jules/layers/narrator/schemas/changes.yml` for schema
2. Determines commit range (previous `to_commit` or bootstrap)
3. Collects commits and changed paths (excluding `.jules/`)
4. Writes `.jules/exchange/changes.yml`

If no non-excluded changes exist, Narrator exits without creating a session.

### 1. Observer Agents (Scheduled)

Each observer:
1. Reads contracts.yml (layer behavior)
2. Uses `.jules/exchange/changes.yml` as a secondary hint only after baseline repository understanding is established
3. Reads role.yml (specialized focus)
4. Reads `.jules/workstations/<role>/perspective.yml`
5. **Skips observations already covered by open requirements (deduplication)**
6. Writes event files under exchange/events/ in the incoming state directory
7. Updates perspective.yml (goals/rules/ignore/log)
8. Publishes changes as a PR (branch naming follows the convention below)


### 2. Decider Agent (Scheduled)

Triage agent:
1. Reads contracts.yml (layer behavior)
2. Reads all event files in the incoming state directory
3. Validates observations (do they exist in codebase?)
4. Merges related events sharing root cause
5. **Merges events into existing requirements when related (updates content)**
6. Creates new requirements for genuinely new problems (placing in requirements directory)
7. **When deep analysis is needed, provides clear rationale in deep_analysis_reason**
8. Moves processed events to the processed state directory defined by the scaffold

**Decider answers**: "Is this real? Should these events merge into one requirement?"

### 3. Planner Agent (On-Demand)

Specifier agent (runs only for `requires_deep_analysis: true`):
1. Reads contracts.yml (layer behavior)
2. Reads target requirement from exchange/requirements/
3. **Reviews deep_analysis_reason to understand scope**
4. Analyzes full system impact and dependency tree
5. Expands requirement with detailed analysis (affected_areas, constraints, risks)
6. Sets requires_deep_analysis to false
7. **Preserves and expands the original rationale with findings**
8. Overwrites the requirement file

**Planner answers**: "What is the full scope of this requirement?"

### 4. Implementation (Via Local Requirement)

Implementation is invoked by running `jlo run implementer` with a local requirement file path. Scheduled workflows may also dispatch implementer based on repository policy.

```bash
# Example: Run implementer with a specific requirement
jlo run implementer .jules/exchange/requirements/auth-inconsistency.yml
```

The implementer reads the requirement content (embedded in prompt) and produces code changes.
The requirement file must exist; missing files fail fast before agent execution.

**Requirement Lifecycle**:
1. A requirement file is selected from `.jules/exchange/requirements/` on the `Worker Branch`.
2. `jlo run implementer` validates the file exists and passes content to the implementer.
3. `jlo run implementer` deletes the requirement file and its source events after dispatching the session.
4. The implementer works on the `Target Branch` and creates a PR for human review.
5. The `sync-jules.yml` workflow keeps `Worker Branch` in sync with the `Target Branch` after merges.

## Observer Continuity

```
Observer creates events in exchange/events/<state>/
       |
       v
Observer updates perspective.yml (goals/rules/ignore/log)
```

## Requirement Lifecycle

- Requirements are stored in a flat directory.
- Open requirements suppress duplicate observations from observers.
- Requirement filenames use stable kebab-case identifiers (for example, `auth-inconsistency.yml`).
- Related events are merged into existing requirements, not duplicated.

## Branch Naming Convention

All agents must create branches using this format:

```
jules-narrator-<id>
jules-observer-<id>
jules-decider-<id>
jules-planner-<id>
jules-implementer-<id>-<short_description>
jules-integrator-<timestamp>-<id>
```

## Testing and Validation

Workflows validate the exchange with `jlo doctor` after each layer execution to detect structural regressions early.

## Pause/Resume

Set the repository pause variable referenced by the workflows to skip scheduled runs.
The default behavior is active; paused behavior is explicit and visible.
