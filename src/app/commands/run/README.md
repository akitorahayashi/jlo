# Run Command Layer Execution

This module implements the execution logic for `jlo run <layer>` and contains side-effecting orchestration for Jules agents (Narrator, Observers, Decider, Planner, Implementer, Innovators, Integrator).

## Architecture

Layer execution follows a strategy pattern through `strategy.rs`. Each layer module in `layer/` implements `LayerStrategy` and is selected by `get_layer_strategy`.

## Execution Modes

Each layer supports real execution and, where applicable, mock execution:

- Real execution creates Jules sessions and performs normal workflow orchestration.
- Mock execution simulates workflow outcomes for validation and writes deterministic outputs.

## Prompt Assembly

Prompt assembly is delegated to `src/domain/prompt_assembly/`. Each layer gathers context, builds a `PromptContext`, and calls `assemble_prompt`.

## Module Responsibilities

- `layer/narrator.rs`: change summarization and narrator session routing
- `layer/observers.rs`: observer role execution and bridge artifact generation
- `layer/decider.rs`: requirement triage and decider workflow routing
- `layer/planner.rs`: requirement expansion flow
- `layer/implementer.rs`: implementation routing and post-run cleanup metadata
- `layer/innovators.rs`: role/task-driven ideation and proposal flow
- `layer/integrator.rs`: implementer-branch discovery and integration routing
- `role_session.rs`: shared role session dispatch helpers
- `mock/mock_execution.rs`: shared mock execution helpers and assets
- `requirement_path.rs`: requirement-path validation
- `strategy.rs`: run-layer strategy contracts and dispatch
