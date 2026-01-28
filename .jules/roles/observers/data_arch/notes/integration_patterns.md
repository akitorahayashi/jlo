# Integration Patterns

## Jules API Integration

**Source:** `.github/actions/jules-invoke/action.yaml`

**Data Flow:**
1.  **Input Aggregation:**
    - User Prompt (`inputs.prompt`)
    - (Optional) Last Commit Content (`git show`)
    - (Optional) Commit History (`git log -20`)
    - Aggregated into `prompt.txt`.

2.  **Payload Construction:**
    - JSON Payload constructed via `jq`.
    - **Schema:**
        ```json
        {
            "prompt": "string",
            "sourceContext": {
                "source": "sources/github/{repo_full_name}",
                "githubRepoContext": {
                    "startingBranch": "{branch_name}"
                }
            },
            "requirePlanApproval": false,
            "automationMode": "AUTO_CREATE_PR"
        }
        ```
    - **Characteristics:**
        - Isomorphic representation of the session request.
        - Encapsulates GitHub context (repository, branch) within `sourceContext`.

3.  **Transport:**
    - POST to `https://jules.googleapis.com/v1alpha/sessions`.
    - Authenticated via `X-Goog-Api-Key`.

**Analysis:**
- **Efficiency:** Using file-based intermediate storage (`prompt.txt`, `jules_payload.json`) avoids shell argument limits.
- **SSOT:** The action serves as the single point of definition for the Jules API contract within the CI/CD pipeline.
