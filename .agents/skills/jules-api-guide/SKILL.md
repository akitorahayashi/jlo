---
name: jules-api-guide
description: Guide for using Jules API and repo settings for automation and customization of sessions, activities, and sources.
---

# Jules API / Repo Settings Skill Guide (v1alpha)

This is a practical, implementation-oriented summary of **the Jules REST API (`https://jules.googleapis.com/v1alpha`)** and the key customization points in **Jules Web repo settings**.

Goals:

* Automate **start → monitor → mid-run instructions → approvals → output retrieval** via the API
* Cover the GUI-side essentials: **setup/snapshot, environment variables, network access, memory**

---

## 1. Basics

* Base URL: `https://jules.googleapis.com/v1alpha`
* Auth: send an API Key in `x-goog-api-key`

```bash
export JULES_API_KEY="..."
curl -H "x-goog-api-key: $JULES_API_KEY" \
  https://jules.googleapis.com/v1alpha/sessions
```

Resource names:

* Session: `sessions/{id}`
* Activity: `sessions/{id}/activities/{id}`
* Source: `sources/{id}`

---

## 2. Concepts (Source / Session / Activity)

### 2.1 Source (connected repo)

* Sources are created by **connecting a GitHub repo in Jules Web UI**
* The API is mainly **list/get**

Typical fields:

* `Source.name`
* `Source.githubRepo.owner / repo / defaultBranch / branches[]`

### 2.2 Session (async task)

Key fields:

* `prompt` (required): instructions
* `sourceContext` (required): target repo/branch
* `title` (optional)
* `requirePlanApproval` (optional, input): manual plan approval
* `automationMode` (optional, input): auto PR
* `state` (output)
* `outputs[]` (output): PR, etc.

### 2.3 Activity (event log)

Monitoring is usually done via Activities.
Common events: `planGenerated / planApproved / userMessaged / agentMessaged / progressUpdated / sessionCompleted / sessionFailed`

---

## 3. State transitions and plan approval

Typical flow:

* `QUEUED → PLANNING → (AWAITING_PLAN_APPROVAL) → IN_PROGRESS → COMPLETED`
* Failure: `FAILED` / Waiting for user: `AWAITING_USER_FEEDBACK` / Paused: `PAUSED`

Plan approval:

* Auto: omit `requirePlanApproval` or set it to false
* Manual: `requirePlanApproval: true` → stops at `AWAITING_PLAN_APPROVAL` → resume with `approvePlan`

---

## 4. API quick reference (v1alpha)

### 4.1 Sources

* `GET /sources`
* `GET /{name=sources/*}`

### 4.2 Sessions

#### Create

* `POST /sessions`

Minimal:

```json
{
  "prompt": "...",
  "sourceContext": {
    "source": "sources/<sourceId>",
    "githubRepoContext": { "startingBranch": "main" }
  }
}
```

Optional:

* `title`
* `requirePlanApproval: true`
* `automationMode: AUTO_CREATE_PR` (auto branch & PR creation)

#### Get / List

* `GET /{name=sessions/*}`
* `GET /sessions?pageSize=&pageToken=`

#### Mid-run instructions

* `POST /{session=sessions/*}:sendMessage`

```json
{ "prompt": "..." }
```

#### Plan approval

* `POST /{session=sessions/*}:approvePlan`

### 4.3 Activities

* `GET /{parent=sessions/*}/activities?pageSize=&pageToken=`
* `GET /{name=sessions/*/activities/*}`

---

## 5. Monitoring (polling) patterns

Robust completion checks (use both):

1. `sessions.get.state` becomes `COMPLETED/FAILED`
2. `activities.list` contains `sessionCompleted/sessionFailed`

Detecting plan approval:

* If `state == AWAITING_PLAN_APPROVAL`, fetch the latest `planGenerated`, show it to a human, then call `approvePlan`

Mid-run interaction:

* `sendMessage` → `userMessaged` → `agentMessaged`

---

## 6. Outputs (outputs / artifacts)

### 6.1 Session.outputs (final outputs)

Typical: `pullRequest` (URL/title/description)

### 6.2 Activity.artifacts (intermediate artifacts)

Typical:

* `changeSet.gitPatch.unidiffPatch` (diff)
* `bashOutput.output` + `exitCode` (command results)
* `media` (base64 data)

Usage:

* Apply locally: `unidiffPatch`
* Debug failures: `bashOutput`

---

## 7. Errors (implementation notes)

* Common: `400/401/403/404/429/500`
* 429: exponential backoff + jitter
* 500: bounded retries
* 400: treat as input errors (no retry by default)

---

## 8. Repo settings (GUI) you must configure

The VM runtime environment is primarily shaped here (often not controllable via API alone).

### 8.1 Setup Script / Snapshot

* Keep initial dependency install + tests minimal and reliable
* Successful `Run and Snapshot` speeds up later runs

### 8.2 Environment Variables

* Register Key/Value at the repo level
* Enable them at session start (changes after start won’t affect existing sessions)
* **No public field is visible for injecting env vars via `sessions.create`** → treat env as UI-managed

### 8.3 Network access

* Enable if external APIs or dependency downloads are required

### 8.4 Knowledge / Memory

* Turn on if you want repo-specific preferences/instructions retained

---

## 9. Minimal automation skeleton

1. Find `sources/{id}` via `sources.list`
2. `sessions.create`
3. Loop:

   * Check `state` via `sessions.get`
   * If `AWAITING_PLAN_APPROVAL`, `activities.list` → show plan → `approvePlan`
   * Stop on `COMPLETED/FAILED`
   * Optionally pull logs/diffs/outputs from `activities.list`

---

## 10. What to change where (cheat sheet)

API:

* Instructions: `prompt`
* Target repo: `sourceContext.source`
* Target branch: `startingBranch`
* Plan approval: `requirePlanApproval` + `approvePlan`
* Auto PR: `automationMode=AUTO_CREATE_PR`
* Mid-run messages: `sendMessage`
* Monitoring: `sessions.get` / `activities.list`

GUI:

* Dependencies/tests/speed: Setup Script + Snapshot
* Secrets: Environment Variables
* External access: Network access
* Persistent repo rules: Knowledge/Memory
