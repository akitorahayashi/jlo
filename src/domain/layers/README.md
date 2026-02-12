# Domain Layers

This module implements the execution logic for the various Jules agents (Narrator, Observers, Decider, Planner, Implementer, Innovators).

## Architecture

The layer implementation follows the **Strategy Pattern**. Each layer implements the `LayerStrategy` trait defined in `strategy.rs`.

### LayerStrategy Trait

The `LayerStrategy` trait defines the contract for executing a layer:

```rust
pub trait LayerStrategy<W> {
    fn execute(
        &self,
        jules_path: &Path,
        options: &RunOptions,
        config: &RunConfig,
        git: &dyn GitPort,
        github: &dyn GitHubPort,
        workspace: &W,
        client_factory: &dyn JulesClientFactory,
    ) -> Result<RunResult, AppError>;
}
```

This abstraction allows the `jlo run` command to be agnostic of the specific layer being executed.

## Execution Modes

Each layer supports two primary execution modes:

### 1. Real Execution (`execute_real`)
- **Trigger**: `jlo run <layer>` (default)
- **Action**: Interacts with the Jules API to generate content.
- **Side Effects**: Creates real branches, PRs, and modifies files in `.jules/`.
- **Implementation**: Typically delegates to a private `execute_real` function that:
    1. Assembles the prompt.
    2. Creates a session with the Jules API.
    3. Handles the API response (e.g., branch creation).

### 2. Mock Execution (`execute_mock`)
- **Trigger**: `jlo run <layer> --mock`
- **Action**: Simulates execution locally without calling the Jules API.
- **Side Effects**:
    - Creates real branches and PRs with synthetic content (for workflow validation).
    - Writes mock output to GitHub Actions outputs (if running in CI).
- **Implementation**: Delegates to a private `execute_mock` function that uses `MockConfig` to generate predictable artifacts.

## Prompt Assembly

Prompts are constructed dynamically using Jinja2 templates located in `.jules/roles/<layer>/<layer>_prompt.j2`.
The logic for assembling prompts is encapsulated in `src/domain/prompt_assembly/`.

Each layer's `execute_real` function is responsible for:
1. Gathering context (git history, file content, etc.).
2. Creating a `PromptContext`.
3. Calling `assemble_prompt`.

## Layer Responsibilities

- **Narrator**: Analyzes git history and updates `.jules/exchange/changes.yml`.
- **Observers**: Analyzes code changes and `changes.yml` to emit events in `.jules/exchange/events/`.
- **Decider**: Aggregates events into requirements in `.jules/exchange/requirements/`.
- **Planner**: Expands complex requirements into detailed plans.
- **Implementer**: Executes requirements by modifying code and creating PRs.
- **Innovators**: Generates ideas and proposals based on their persona.

## Testing

- **Unit Tests**: Co-located in the same file as the layer implementation (e.g., `narrator.rs`).
- **Integration Tests**: `tests/cli_commands.rs` verifies the end-to-end flow using the CLI.
