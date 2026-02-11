# Jules Workflow Reproduction Guide

This document describes the repository state required to reproduce the Jules workflow in other projects.

## Required Configuration

Configure repository variables and secrets referenced by the workflow kit (see `.github/workflows/jules-*.yml`).
Branch protection on `JULES_WORKER_BRANCH` must require the workflow status checks and allow auto-merge.

### Required Secrets and Permissions

- `JULES_API_KEY`: Jules API key.
- `JLO_BOT_TOKEN`: Automation token for repository operations (checkout/push/labels/automerge).
- `JULES_LINKED_GH_PAT`: Personal access token used only by `jules-pr-summary-request.yml`.
  - This value is a PAT, not a generic GitHub token label.
  - It authenticates as the GitHub account linked to the `JULES_API_KEY` principal.
  - It is distinct from `JLO_BOT_TOKEN` (same value is invalid).

Minimum token permissions:

- Fine-grained PAT for `JLO_BOT_TOKEN`:
  - `Contents: Read and write`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Fine-grained PAT for `JULES_LINKED_GH_PAT`:
  - `Contents: Read`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Classic PAT alternative (private repository): `repo` scope.

`gh auth token` output is valid only when it is the PAT for the correct linked GitHub account and has the required permissions above.

## Required Files

Install the control plane and workflow kit with `jlo init --remote` (or `--self-hosted`) to populate `.jlo/` and `.github/` assets.

The kit layout follows these patterns:

- `.github/workflows/jules-*.yml`
- `.github/actions/` (Jules composite actions)

### Review Configuration

If you use automated review tools, configure them to suppress reviews on Jules-managed PRs while keeping reviews active for human Implementer PRs.

### .jlo/ (control plane)

The `.jlo/` directory is the user-facing intent overlay, created by `jlo init` on `JLO_TARGET_BRANCH`. It contains role definitions, version pins, and configuration.

### .jules/ (runtime)

The `.jules/` directory on `JULES_WORKER_BRANCH` is assembled automatically by the workflow bootstrap job. Users never edit it directly.

## Repository State

- The `JLO_TARGET_BRANCH` branch contains `.jlo/` and `.github/` (installed by `jlo init`).
- The `JULES_WORKER_BRANCH` branch is created and maintained by workflow automation (bootstrap job).
- The git identity configured by the workflow kit matches the target repository's bot account.
- The bot account used by workflows has write access to the repository.
- Auto-review tools are configured for on-demand review only for Jules-managed PRs.

## Workflow Execution Flow

The orchestration workflow under `.github/workflows/jules-*.yml` runs the layers in sequence, producing agent branches according to workflow rules.

- Observer/Decider/Planner: Only `.jules/` changes, auto-merge
- Implementer: Source code changes, human review required

## Self-hosted Runners

The self-hosted workflow kit uses `runs-on: self-hosted` and installs `jlo` into the runner temp directory, adding it to the workflow PATH without requiring `sudo`.
Self-hosted runners must provide the commands referenced by the workflows; treat the workflow templates as the authoritative source of required tooling.
The installer detects OS/architecture and fails fast if the release assets do not support the runner.

## Troubleshooting

### Auto-merge Fails

- Repository settings, branch protection, and permissions align with the requirements referenced by the Jules workflows.
