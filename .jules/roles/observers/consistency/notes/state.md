# Consistency Observer State

## Last Update
Date: 2026-02-01

## Structural Analysis
The project maintains documentation in `README.md` (user-facing) and `AGENTS.md` (developer/agent-facing). Discrepancies often arise between these documents and the implementation sources of truth:
- `src/main.rs` (CLI command structure)
- `Cargo.toml` (Dependencies)
- `src/domain/run_config.rs` (Configuration defaults)

## Active Patterns
- **Doc Rot**: Documentation tends to retain references to removed features (e.g., clipboard, arboard) or legacy configs.
- **CLI Drift**: New flags (e.g., `--adopt-managed`) are added to the code but not immediately reflected in `README.md`.
- **Help Text Inaccuracy**: CLI help text (clap derive) sometimes describes aspirational or copy-pasted behavior that contradicts specific logic checks.

## Recent Findings
- Identified undocumented `--adopt-managed` flag.
- Identified stale clipboard references.
- Identified incomplete dependency lists.
- Identified API URL configuration mismatch.
- Identified misleading template command help.
