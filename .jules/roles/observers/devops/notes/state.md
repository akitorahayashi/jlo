# DevOps State

## CI/CD Architecture
The project uses GitHub Actions for CI/CD. The workflows are modularized with reusable workflows for linting, building, and testing.

- **Orchestration**: `ci-workflows.yml` orchestrates standard CI checks.
- **Build**: `build.yml` builds debug and release binaries but does not upload artifacts.
- **Release**: `release.yml` builds cross-platform binaries and publishes a release.
- **Agent**: `jules-workflows.yml` manages the Jules agent execution.

## Observations (2026-01-31)
Analyzed `.github/workflows/` and found several anti-patterns:

1.  **Toolchain Inconsistency**: CI runs on `stable` (via default in `setup` action), but Release runs on `1.90.0`.
2.  **Supply Chain Risk**: Actions are pinned by mutable tags (`v4`, `v1`) rather than immutable SHAs.
3.  **Implicit Permissions**: Most workflows lack explicit permission scopes, defaulting to repository settings.
4.  **Artifact Rebuilding**: `release.yml` rebuilds binaries instead of promoting verified artifacts from a build pipeline.

## Recommendations
- Pin toolchains in `setup/action.yml` or enforce consistency across workflows.
- Switch to SHA pinning for all actions.
- Add `permissions: read-all` (or specific scopes) to all workflows.
- Refactor the release pipeline to download artifacts from a successful build workflow run.
