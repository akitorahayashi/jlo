# jlo Design Overview

`jlo` manages the `.jules/` scaffolding for agent configuration and assets, and orchestrates the complete autonomous lifecycle via GitHub Actions.

| Component | Responsibility |
|-----------|----------------|
| jlo | Scaffold installation, versioning, prompt asset management |
| GitHub Actions | Orchestration: cron triggers, matrix execution, auto-merge control |
| Jules API | Execution: code analysis, artifact generation, branch/PR creation |

## Documentation

- **[Documentation Index](docs/README.md)**: The central index for all architectural decisions, operational guides, and design principles.
- **[Development Context](src/AGENTS.md)**: Specific instructions for Rust CLI development and verification.
