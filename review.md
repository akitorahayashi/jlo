# Implementation Review: Jules E2E Pipeline

## Verdict: Scope Misalignment

The implementation addresses **schema conformance** rather than **workflow flow validation**. This is a fundamental misunderstanding of the requirement.

## What Was Requested

> observer → decider → planner のフロー検証

The goal: verify that the pipeline stages interact correctly—events trigger deciders, deciders produce issues, planners expand issues, and the orchestration propagates correctly through `workflow_dispatch` or file-based handoffs.

## What Was Delivered

1. **`validate-jules-schema/`** - A Python script checking YAML structure against templates
2. **`jules-e2e-pipeline.yml`** - A mock generator that creates fake artifacts and validates their schema

### Critical Flaws

| Issue | Explanation |
|-------|-------------|
| **Wrong abstraction level** | Schema validation checks "is this YAML valid?" not "does the pipeline behave correctly?" |
| **Simulated flow is not flow** | Writing mock files sequentially does not test workflow interactions. It's a shell script, not a workflow graph. |
| **No actual orchestration tested** | `workflow_dispatch` chaining, job dependencies, and failure propagation are untested. |
| **Unnecessary complexity** | 200+ lines of Python for something the YAML templates implicitly define. If a template is invalid YAML, `yaml.safe_load` fails—no custom validator needed. |
| **Scope creep** | Schema validation was proposed as "Phase 1" in plan.md but it's not a prerequisite for flow testing. |

## What Should Be Done

### Actual Flow Validation Requirements

1. **Observer → Decider handoff**: After `jlo run observers`, does `.jules/exchange/events/` contain files? After `jlo run deciders`, are those files deleted and issues created?

2. **Workflow chaining**: Does the completion of one layer trigger the next? This requires testing `workflow_dispatch` invocation or job dependencies.

3. **Failure modes**: What happens when an observer produces invalid output? Does the decider reject it properly?

### Realistic Approaches

| Approach | Feasibility | Coverage |
|----------|-------------|----------|
| **Mock Jules API in jlo CLI** | Medium | Tests CLI logic, not GitHub Actions |
| **Dry-run with file assertions** | High | Tests artifact existence without API |
| **Integration test with real Jules** | Low frequency | Full coverage but expensive |

### Minimum Viable Flow Test

```yaml
jobs:
  test-flow:
    steps:
      - name: Seed mock event
        run: |
          mkdir -p .jules/exchange/events/bugs
          echo "..." > .jules/exchange/events/bugs/test.yml

      - name: Run decider (dry-run or mock)
        run: jlo run deciders --dry-run

      - name: Verify handoff
        run: |
          # Event should be deleted (or marked processed)
          # Issue should exist
          test -f .jules/exchange/issues/*.yml
          test ! -f .jules/exchange/events/bugs/test.yml
```

This is 20 lines vs 200+ lines of Python.

## Recommendations

1. **Delete `validate-jules-schema/`** - Schema enforcement is implicit in template usage. If needed, a 10-line shell script suffices.

2. **Rewrite `jules-e2e-pipeline.yml`** - Focus on artifact state transitions, not schema structure.

3. **Define flow invariants explicitly** - In contracts.yml or a dedicated test spec:
   - "After deciders run, events/ is empty"
   - "After planners run, no issues have requires_deep_analysis: true"

4. **Test actual workflow mechanics** - PR creation, auto-merge trigger, workflow_dispatch propagation. The existing `jules-e2e-test.yml` already does some of this.

## Conclusion

The implementation conflated "validate artifact format" with "validate pipeline behavior." Format validation is a trivial concern; flow validation is the actual challenge. The resources spent on schema checking should be redirected to testing the state machine: events → issues → expanded issues → (optionally) GitHub Issues.
