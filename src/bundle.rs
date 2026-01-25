//! Embedded bundle content for `.jules/.jo/` directory.
//!
//! This module provides static content for policy documents, templates, and
//! starter files that jo deploys into a workspace.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Bundle entry with path relative to `.jules/` and content.
pub struct BundleEntry {
    pub path: &'static str,
    pub content: &'static str,
}

/// Returns all jo-managed files that belong under `.jules/.jo/`.
pub fn jo_managed_files() -> Vec<BundleEntry> {
    vec![
        // Policy files
        BundleEntry { path: ".jo/policy/contract.md", content: POLICY_CONTRACT },
        BundleEntry { path: ".jo/policy/layout.md", content: POLICY_LAYOUT },
        BundleEntry { path: ".jo/policy/run-bootstrap.md", content: POLICY_RUN_BOOTSTRAP },
        BundleEntry { path: ".jo/policy/run-output.md", content: POLICY_RUN_OUTPUT },
        BundleEntry { path: ".jo/policy/role-boundaries.md", content: POLICY_ROLE_BOUNDARIES },
        BundleEntry { path: ".jo/policy/exchange.md", content: POLICY_EXCHANGE },
        BundleEntry { path: ".jo/policy/decisions.md", content: POLICY_DECISIONS },
        // Template files
        BundleEntry { path: ".jo/templates/session.md", content: TEMPLATE_SESSION },
        BundleEntry { path: ".jo/templates/decision.md", content: TEMPLATE_DECISION },
        BundleEntry {
            path: ".jo/templates/weekly-synthesis.md",
            content: TEMPLATE_WEEKLY_SYNTHESIS,
        },
    ]
}

/// Returns the START_HERE.md content for the `.jules/` root.
pub fn start_here_content() -> &'static str {
    START_HERE
}

/// Returns starter org files for `.jules/org/`.
pub fn org_files() -> Vec<BundleEntry> {
    vec![
        BundleEntry { path: "org/north_star.md", content: ORG_NORTH_STAR },
        BundleEntry { path: "org/constraints.md", content: ORG_CONSTRAINTS },
        BundleEntry { path: "org/current_priorities.md", content: ORG_CURRENT_PRIORITIES },
    ]
}

/// Returns the charter template for a new role.
pub fn role_charter_template() -> &'static str {
    ROLE_CHARTER
}

/// Returns the direction template for a new role.
pub fn role_direction_template() -> &'static str {
    ROLE_DIRECTION
}

/// Returns the session template content with placeholders.
pub fn session_template() -> &'static str {
    TEMPLATE_SESSION
}

/// Static lookup map of path -> content for jo-managed files.
static JO_MANAGED_MAP: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| jo_managed_files().into_iter().map(|e| (e.path, e.content)).collect());

/// Returns a reference to the lookup map of path -> content for jo-managed files.
pub fn jo_managed_map() -> &'static HashMap<&'static str, &'static str> {
    &JO_MANAGED_MAP
}

// =============================================================================
// Policy Content
// =============================================================================

const POLICY_CONTRACT: &str = r#"# .jules Workspace Contract

## Purpose

The `.jules/` directory is repository-local organizational memory and a workflow contract for scheduled LLM agents and humans. It persists direction, decisions, and per-role session outputs so each scheduled run starts fresh while still regaining context by reading `.jules/`.

## Principles

1. **Immutable session outputs** — Each run creates a new session file; no in-place edits.
2. **Scheduled tasks are read-only for product code** — Agents write only under `.jules/`.
3. **Source-of-truth documents prevent drift** — `org/` holds canonical direction.
4. **Roles are decision functions** — Generic vocabulary, not domain-specific titles.
5. **jo owns `.jo/`** — Files under `.jules/.jo/` are managed by jo and may be overwritten.

## Ownership

| Path | Owner | Notes |
|------|-------|-------|
| `.jules/.jo/` | jo | Overwritten by `jo update` |
| `.jules/.jo-version` | jo | Version marker |
| `.jules/org/` | Human | Source-of-truth documents |
| `.jules/decisions/` | Human/Agent | Decision records |
| `.jules/roles/` | Human/Agent | Role outputs |
| `.jules/exchange/` | Agent | Inter-role communication |
| `.jules/synthesis/` | Agent | Periodic synthesis |
| `.jules/state/` | Agent | Machine-readable state |
"#;

const POLICY_LAYOUT: &str = r#"# Directory Layout Reference

```text
.jules/
  START_HERE.md              # Entry point for navigating the workspace
  .jo-version                # jo version that last deployed .jo/
  .jo/                       # jo-managed policy and templates
    policy/
      contract.md
      layout.md
      run-bootstrap.md
      run-output.md
      role-boundaries.md
      exchange.md
      decisions.md
    templates/
      session.md
      decision.md
      weekly-synthesis.md
  org/                       # Source-of-truth direction
    north_star.md
    constraints.md
    current_priorities.md
  decisions/                 # Decision records by year
    YYYY/
      YYYY-MM-DD_<slug>.md
  roles/                     # Per-role workspaces
    <role_id>/
      charter.md
      direction.md
      sessions/
        YYYY-MM-DD/
          HHMMSS_<slug>.md
  exchange/                  # Inter-role communication
    inbox/
      <role_id>/
    threads/
      <thread_id>/
  synthesis/                 # Periodic synthesis outputs
    weekly/
      YYYY-WW.md
  state/                     # Machine-readable state
    lenses.json
    open_threads.json
```
"#;

const POLICY_RUN_BOOTSTRAP: &str = r#"# Run Bootstrap Instructions

When a scheduled agent run begins:

1. **Read `START_HERE.md`** — Understand the workspace structure.
2. **Read `org/`** — Load current direction, constraints, and priorities.
3. **Read your role's `charter.md`** — Understand your decision function.
4. **Read your role's `direction.md`** — Load role-specific guidance.
5. **Scan `exchange/inbox/<your_role_id>/`** — Check for messages from other roles.
6. **Read recent sessions** — Gain context from prior runs.

## Output Rules

- Write session output to `roles/<role_id>/sessions/YYYY-MM-DD/HHMMSS_<slug>.md`.
- Never modify files outside `.jules/`.
- Never modify `org/` or `decisions/` — those are human-managed.
"#;

const POLICY_RUN_OUTPUT: &str = r#"# Run Output Conventions

## Session Files

Session files capture the output of a single agent run.

### Naming Convention

```
roles/<role_id>/sessions/YYYY-MM-DD/HHMMSS_<slug>.md
```

- `YYYY-MM-DD` — ISO date of the session
- `HHMMSS` — 24-hour timestamp (UTC recommended)
- `<slug>` — Brief description (lowercase, hyphens)

### Session File Structure

```markdown
# Session: <slug>

**Role:** <role_id>
**Date:** YYYY-MM-DD HH:MM:SS UTC
**Trigger:** <what initiated this run>

## Context

<What was read and understood before starting>

## Analysis

<Reasoning and observations>

## Outputs

<Concrete deliverables or recommendations>

## Next Steps

<Suggested follow-up actions>
```

## Message Files

For communication between roles, write to `exchange/inbox/<target_role>/`.

```
exchange/inbox/<role_id>/YYYY-MM-DD_HHMMSS_<slug>.md
```
"#;

const POLICY_ROLE_BOUNDARIES: &str = r#"# Role Boundaries

## What is a Role?

A **role** is a stable decision function, not a job title or domain-specific persona.

## Generic Vocabulary

Prefer generic role identifiers:

| Role ID | Decision Function |
|---------|-------------------|
| `value` | Prioritize based on user/business value |
| `quality` | Ensure correctness, maintainability, polish |
| `feasibility` | Assess technical viability and effort |
| `risk` | Identify and mitigate risks |
| `synthesis` | Integrate perspectives, resolve conflicts |

## Anti-patterns

❌ Creating roles for every domain (e.g., "game-designer", "network-engineer")
❌ Issue-specific roles (e.g., "fix-login-bug")
❌ Overlapping responsibilities between roles

## Guidelines

- **Reuse over creation** — Use existing roles before creating new ones.
- **Composition over specialization** — Express issue-specific focus in session content.
- **Scheduler maps display names** — Domain-specific labels exist in the scheduler, not here.
"#;

const POLICY_EXCHANGE: &str = r#"# Inter-Role Exchange

## Purpose

The `exchange/` directory enables asynchronous communication between roles.

## Structure

```text
exchange/
  inbox/
    <role_id>/              # Incoming messages for a role
      YYYY-MM-DD_HHMMSS_<slug>.md
  threads/
    <thread_id>/            # Multi-message conversations
      README.md             # Thread context
      YYYY-MM-DD_HHMMSS_<slug>.md
```

## Inbox Messages

When one role needs input from another:

1. Write a message to `exchange/inbox/<target_role>/`.
2. Include context, question, and expected response format.
3. The target role checks their inbox at run start.

## Threads

For ongoing conversations:

1. Create a thread directory with a unique ID.
2. Add `README.md` describing the thread purpose.
3. Participants add timestamped messages to the thread.

## Message Format

```markdown
# Message: <subject>

**From:** <source_role>
**To:** <target_role>
**Date:** YYYY-MM-DD HH:MM:SS UTC

## Context

<Background for this message>

## Request

<What you need from the recipient>

## Expected Response

<Format or type of response you need>
```
"#;

const POLICY_DECISIONS: &str = r#"# Decision Records

## Purpose

The `decisions/` directory preserves significant decisions with rationale.

## Structure

```text
decisions/
  YYYY/
    YYYY-MM-DD_<slug>.md
```

## When to Record a Decision

- Architectural choices
- Policy changes
- Scope decisions
- Trade-off resolutions

## Decision Record Format

```markdown
# Decision: <title>

**Date:** YYYY-MM-DD
**Status:** proposed | accepted | deprecated | superseded
**Deciders:** <who made this decision>

## Context

<What prompted this decision>

## Decision

<The decision made>

## Rationale

<Why this decision was made>

## Consequences

<Expected outcomes, both positive and negative>

## Alternatives Considered

<Other options and why they were not chosen>
```
"#;

// =============================================================================
// Template Content
// =============================================================================

const TEMPLATE_SESSION: &str = r#"# Session: {{slug}}

**Role:** {{role_id}}
**Date:** {{date}} {{time}} UTC
**Trigger:** <describe what initiated this session>

## Context

<What was read and understood before starting>

## Analysis

<Reasoning and observations>

## Outputs

<Concrete deliverables or recommendations>

## Next Steps

<Suggested follow-up actions>
"#;

const TEMPLATE_DECISION: &str = r#"# Decision: {{title}}

**Date:** {{date}}
**Status:** proposed
**Deciders:** <who made this decision>

## Context

<What prompted this decision>

## Decision

<The decision made>

## Rationale

<Why this decision was made>

## Consequences

<Expected outcomes, both positive and negative>

## Alternatives Considered

<Other options and why they were not chosen>
"#;

const TEMPLATE_WEEKLY_SYNTHESIS: &str = r#"# Weekly Synthesis: {{week}}

**Period:** {{start_date}} to {{end_date}}
**Author:** synthesis role

## Summary

<High-level summary of the week's activity>

## Key Decisions

<Decisions made this week>

## Progress by Role

### value
<Activity summary>

### quality
<Activity summary>

### feasibility
<Activity summary>

## Open Threads

<Ongoing discussions requiring attention>

## Next Week Focus

<Priorities for the coming week>
"#;

// =============================================================================
// Root and Org Content
// =============================================================================

const START_HERE: &str = r#"# .jules Workspace

Welcome to the `.jules/` organizational memory workspace.

## Quick Navigation

- **[org/](org/)** — Source-of-truth documents (north star, constraints, priorities)
- **[decisions/](decisions/)** — Decision records
- **[roles/](roles/)** — Per-role workspaces with sessions
- **[exchange/](exchange/)** — Inter-role communication
- **[synthesis/](synthesis/)** — Periodic synthesis outputs
- **[state/](state/)** — Machine-readable state

## For Agents

Read `.jo/policy/run-bootstrap.md` for bootstrap instructions.

## For Humans

Edit files in `org/` to set direction. Create roles with `jo role <id>`.

## jo-managed Files

Files under `.jo/` are managed by jo and will be updated when jo is upgraded.
Run `jo status` to check for updates. Run `jo update` to apply them.
"#;

const ORG_NORTH_STAR: &str = r#"# North Star

<Define the ultimate vision and purpose of this project>

## Vision

<What does success look like?>

## Mission

<How will you achieve the vision?>

## Core Values

<Principles that guide decision-making>
"#;

const ORG_CONSTRAINTS: &str = r#"# Constraints

<Define the boundaries and limitations that shape decisions>

## Technical Constraints

<Technology, platform, or architectural boundaries>

## Resource Constraints

<Time, budget, team limitations>

## External Constraints

<Regulatory, market, or dependency constraints>
"#;

const ORG_CURRENT_PRIORITIES: &str = r#"# Current Priorities

<What are the most important things to focus on right now?>

## This Week

1. <Priority 1>
2. <Priority 2>
3. <Priority 3>

## This Month

1. <Priority 1>
2. <Priority 2>
3. <Priority 3>

## Blocked or Deferred

<Items that are on hold and why>
"#;

// =============================================================================
// Role Templates
// =============================================================================

const ROLE_CHARTER: &str = r#"# {{role_id}} Role Charter

## Decision Function

<What decisions does this role make?>

## Scope

<What is in scope for this role?>

## Inputs

<What information does this role consume?>

## Outputs

<What does this role produce?>

## Interactions

<How does this role interact with other roles?>
"#;

const ROLE_DIRECTION: &str = r#"# {{role_id}} Direction

## Current Focus

<What is this role currently focused on?>

## Guidelines

<Specific guidance for this role's sessions>

## Recent Context

<Important context from recent sessions>
"#;
