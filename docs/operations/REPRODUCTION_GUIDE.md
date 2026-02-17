# Jules Workflow Reproduction Guide

This document describes the repository state required to reproduce Jules workflow behavior in another repository.

## Required Configuration

Configure repository secrets and variables referenced by the workflow kit.
Branch protection on `JULES_WORKER_BRANCH` must require workflow checks and allow auto-merge.

## Jules GUI Initial Setup

`jlo init --remote` (or `--self-hosted`) generates setup artifacts in the control plane:

- `.jlo/setup/install.sh`
- `.jlo/setup/vars.toml`
- `.jlo/setup/secrets.toml`

Register `.jlo/setup/install.sh` in Jules GUI as the VM initial setup script.
Configure values from `.jlo/setup/secrets.toml` in Jules GUI secret settings.
Non-secret pinned values from `.jlo/setup/vars.toml` stay in-repo for reproducible setup.

### Required Secrets and Permissions

- `JULES_API_KEY`: Jules API key.
- `JLO_BOT_TOKEN`: automation token for checkout/push/merge operations.
- `JULES_LINKED_GH_PAT`: token used by implementer PR metadata processing in `jules-implementer-pr.yml`.
- `GH_TOKEN`: token configured in Jules GUI and consumed inside the Jules VM by `gh`.

Minimum token permissions:

- Fine-grained PAT for `JLO_BOT_TOKEN`:
  - `Contents: Read and write`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Fine-grained PAT for `JULES_LINKED_GH_PAT`:
  - `Contents: Read`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
- Fine-grained token for `GH_TOKEN` (Jules VM runtime):
  - `Contents: Read and write`
  - `Pull requests: Read and write`
  - `Issues: Read and write`
  - Equivalent capabilities are acceptable when org policy uses a different fine-grained model.
- Classic PAT alternative (private repository): `repo` scope.

## Required Files

Install control-plane and workflow kit with `jlo init --remote` (or `--self-hosted`).

Expected workflow outputs:

- `.github/workflows/jules-scheduled-workflows.yml`
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
- bootstrap-time target-to-worker sync via bootstrap PR into `JULES_WORKER_BRANCH`
- implementer PR metadata processing
- worker PR doctor validation and auto-merge processing

Auto-merge remains policy-qualified: layer/publish/cleanup PRs are `.jules/`-scoped, and bootstrap sync PRs use the dedicated bootstrap branch policy.

## Self-hosted Runners

Self-hosted mode renders `runs-on: self-hosted` and installs `jlo` into runner temp PATH.
Runners must provide tools referenced by workflow templates.

## Troubleshooting

### Auto-merge Fails

- Confirm repository settings allow auto-merge.
- Confirm branch protection requires workflow checks on `JULES_WORKER_BRANCH`.
- Confirm `JLO_BOT_TOKEN` permissions are sufficient for merge operations.

## Integrator Token Contract

The integrator workflow itself does not require runner-side `JULES_LINKED_GH_PAT` wiring.
`gh` runtime authentication for integrator behavior is provided in the Jules VM via `GH_TOKEN`.
