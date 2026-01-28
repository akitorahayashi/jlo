# /wc-t-dc - Work on tasks critically (with Tests & Docs)

Perform a comprehensive planning process that includes test and documentation strategy, and then immediately execute the resulting plan.

This command is for tasks that require changes to tests or documentation in addition to code.

**Important:** Ensure the entire workflow completes fully without premature termination. Do not stop mid-step; complete all steps in the workflow before proceeding.

## Workflow

1.  **Analyze Goal:** Study the user's request and any existing plan.
2.  **Critically Review Scope:** Critically review the scope of edits needed, considering what might be missing in the plan, and ensure sufficient editing is contemplated for the goal.
3.  **Audit Tests:** Review test structure to identify required additions or updates.
4.  **Identify Documentation:** Determine which existing documentation (README.md, AGENTS.md, docs/) will need updates—follow project documentation culture.
5.  **Create Revised Plan:** Create `.mx/revised_tasks.md` to include all required deliverables for code, tests, and documentation. Note that backward compatibility should not be considered, and when migrating to the new system, ensure that the transition is simple and free of technical debt.

**Critical Emphasis:** Do not stop after creating the revised plan. Immediately proceed to the implementation and fully implement all tasks in `.mx/revised_tasks.md` until completion. The absolute goal is to finish the entire implementation, not just planning. Never end with just planning—always execute the full workflow.

6.  **Implement:** Execute all changes defined in the comprehensive plan, including code, tests, and documentation updates. Complete the entire implementation without interruption before moving to verification.
7.  **Verify:** Run tests and validate that all parts of the plan are complete.

Existing plan:
(ここにタスク内容を埋め込み)
