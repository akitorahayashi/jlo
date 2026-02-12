# Skills Framework

## Purpose

This document defines the standard procedure and schema for creating and maintaining skills in `.agents/skills/`.

## Directory Structure

Each skill resides in its own directory under `.agents/skills/`:

```
.agents/skills/<skill-name>/
├── SKILL.md
└── agents/
    └── openai.yaml
```

- `<skill-name>`: Kebab-case identifier for the skill (e.g., `create-jlo-innovator`).
- `SKILL.md`: The procedure definition and instructions for the skill.
- `agents/openai.yaml`: Configuration for the skill interface.

## SKILL.md Standard

The `SKILL.md` file defines the skill's behavior. It must include frontmatter and specific sections.

### Frontmatter

```yaml
---
name: <skill-name>
description: <short description of what the skill does>
---
```

### Sections

1.  **Core Objective**: A clear statement of the skill's goal.
2.  **Output Contract**: Defines the target file and required schema/shape.
3.  **Design Workflow** (or **Procedure**): Step-by-step instructions for the skill execution.
4.  **Boundary Rules**: Constraints on what the skill should *not* do.
5.  **Anti-Pattern Checks**: Specific checks to avoid common mistakes.

### Explicit Branching

Skills often handle both creation and review tasks. This branching must be explicit in the procedure.

**Example Structure:**

```markdown
## Design Workflow (Creation)

1. Step 1...
2. Step 2...

## Review Mode

When reviewing an existing artifact:
1. Check 1...
2. Check 2...
```

The prompt (in `openai.yaml`) should instruct the LLM to choose the appropriate mode based on user input.

## Schema Configuration (`agents/openai.yaml`)

This file defines how the skill is exposed to the interface.

```yaml
interface:
  display_name: "<Human-readable Name>"
  short_description: "<Concise description of the skill>"
  default_prompt: "Use $<skill-name> to <action>."
```

### Fields

- `display_name`: The name shown in the UI/CLI.
- `short_description`: A brief summary of the skill's capability.
- `default_prompt`: The default instruction given to the agent when this skill is invoked. It should reference the skill by name (e.g., `$skill-name`).

## Skill Creation Guide

1.  Create a new directory: `.agents/skills/<new-skill-name>/`.
2.  Add `agents/openai.yaml` with the interface configuration.
3.  Add `SKILL.md` with the procedure definition.
    - Ensure frontmatter is present.
    - Define clear Core Objective and Output Contract.
    - Explicitly define Creation and Review workflows if applicable.
