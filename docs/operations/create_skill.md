# Create a New Skill

Skills are specialized capabilities provided to agents (like Innovators or Observers) to perform specific tasks. They are defined as directories within `.agents/skills/`.

## Directory Structure

To create a new skill, create a directory in `.agents/skills/` with a unique ID (e.g., `my-new-skill`). Inside, you must have the following structure:

```
.agents/skills/<skill-id>/
├── SKILL.md             # The skill definition and prompt
└── agents/
    └── openai.yaml      # Interface configuration for LLMs
```

## SKILL.md Schema

The `SKILL.md` file defines the skill's purpose and the prompt that the agent will use.

### Frontmatter

The file must start with YAML frontmatter containing:

- `name`: The unique identifier for the skill (should match the directory name).
- `description`: A brief summary of what the skill does.

### Content

The body of the file is the prompt template. It should clearly describe the objective, output format, and constraints for the task.

**Template:**

```markdown
---
name: <skill-id>
description: <Brief description of the skill>
---

# <Skill Title>

## Objective

<Describe the goal of this skill.>

## Output Format

<Describe the expected output structure.>

## Constraints

- <Constraint 1>
- <Constraint 2>
```

## agents/openai.yaml Schema

The `agents/openai.yaml` file configures how the skill is exposed to the LLM via the tool interface.

### Schema

- `interface`: The main configuration block.
  - `display_name`: Human-readable name for the skill.
  - `short_description`: A concise description of the skill's function.
  - `default_prompt`: The default instruction to trigger the skill. It can reference the skill ID as a variable (e.g., `$skill-id`).

**Template:**

```yaml
interface:
  display_name: "<Skill Display Name>"
  short_description: "<Concise description>"
  default_prompt: "Use $<skill-id> to <action>."
```

## Registration

1. Create the directory `.agents/skills/<skill-id>/`.
2. Add `SKILL.md` with the required frontmatter and content.
3. Create the `agents/` subdirectory.
4. Add `agents/openai.yaml` with the interface configuration.
5. The skill will be automatically available to agents that have access to the `.agents/skills/` directory.

## Best Practices

- **Conciseness**: Keep the `SKILL.md` content focused and concise to save tokens.
- **Clarity**: Ensure the `default_prompt` clearly instructs the LLM on when and how to use the skill.
- **Testing**: Test the skill with a representative agent to ensure it produces the desired output.
