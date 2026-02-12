# Jules Workflow Reproduction Guide

This document describes the repository state required to reproduce Jules workflow behavior in another repository.

## Required Configuration

Configure repository secrets and variables referenced by the workflow kit.
Branch protection on `JULES_WORKER_BRANCH` must require workflow checks and allow auto-merge.

### Required Secrets and Permissions

- `JULES_API_KEY`: Jules API key.
- `JLO_BOT_TOKEN`: automation token for checkout/push/merge operations.
- `JULES_LINKED_GH_TOKEN`: token used by implementer PR metadata processing in `jules-scheduled-workflows.yml`.

Minimum token permissions:

- Fine-grained PAT for `JLO_BOT_TOKEN`:
  - `Contents: Read and write`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Fine-grained PAT for `JULES_LINKED_GH_TOKEN`:
  - `Contents: Read`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Classic PAT alternative (private repository): `repo` scope.

## Required Files

Install control-plane and workflow kit with `jlo init --remote` (or `--self-hosted`).

Expected workflow outputs:

- `.github/workflows/jules-scheduled-workflows.yml`
- `.github/workflows/jules-mock-cleanup.yml`
- `.github/actions/*`

### Review Configuration

If automated review tools are enabled, configure them to avoid blocking Jules-managed PR flow while preserving review quality for human implementer PRs.

## Repository State

- `JLO_TARGET_BRANCH` contains `.jlo/` and `.github/`.
- `JULES_WORKER_BRANCH` is managed by workflow automation.
- Workflow bot identities have write access to the repository.

## Workflow Execution Flow

`jules-scheduled-workflows.yml` orchestrates:

- schedule/dispatch/call layer execution
- target-branch sync to worker branch
- implementer PR metadata processing
- worker PR doctor validation and auto-merge processing

Auto-merge remains limited to policy-qualified `.jules/`-scoped Jules PRs.

## Self-hosted Runners

Self-hosted mode renders `runs-on: self-hosted` and installs `jlo` into runner temp PATH.
Runners must provide tools referenced by workflow templates.

## Troubleshooting

### Auto-merge Fails

- Confirm repository settings allow auto-merge.
- Confirm branch protection requires workflow checks on `JULES_WORKER_BRANCH`.
- Confirm `JLO_BOT_TOKEN` permissions are sufficient for merge operations.
