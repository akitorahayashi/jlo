# Integrator
Layer responsible for merging implemented requirements into a cohesive integration branch.

## Interface
- **Input**: Remote branches matching `jules-implementer-*`.
- **Output**: Single integration branch (`jules-integrator-<timestamp>-<id>`) and one pull request.
- **Execution**: `jlo run integrator`

## Process
1. **Discovery**: Enumerates remote branches on origin matching the implementer branch prefix.
2. **Context Retrieval**: For each branch, fetches the associated Pull Request (summary, comments, reviews).
3. **Integration**: Merges each candidate branch into the integration branch.
4. **Conflict Resolution**: Resolves conflicts contextually based on repository conventions and architecture consistency.
5. **Completion**: Pushes the integration branch and opens a single Pull Request.

## Constraints
- **Scope**: Processes every discovered candidate branch end-to-end without pausing.
- **Naming**: Output branch format is `jules-integrator-<timestamp>-<id>`.
- **No Local State**: Does not depend on `.mx/branches/*.md` or pre-generated local summaries.
- **Failures**: Fails explicitly if zero candidate branches exist or if PR retrieval fails.

## Resources
- **Schema**: N/A (Operates on Git branches and GitHub PRs directly).
- **Tasks**:
  - `contracts.yml`: Defines the integration policy and discovery rules.
