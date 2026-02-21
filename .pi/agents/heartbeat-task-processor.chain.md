---
name: heartbeat-task-processor
description: Heartbeat chain: pick next .agents/tasks work item, implement it, then run fresh-eyes verification with auto-fix and retest.
---

## scout

Heartbeat Explore phase.

Goal: choose the single next task in .agents/tasks that makes the most implementation sense right now.

Process:
1) Scan .agents/tasks recursively for *.code-task.md files.
2) Exclude tasks that are clearly complete (frontmatter/status/body indicates completed/done/closed).
3) Prefer tasks with explicit sequencing/dependencies satisfied (e.g., numeric prefixes 01,02,... and dependency notes).
4) Pick ONE task with best readiness + impact.

Output requirements:
- Print a short decision summary.
- Include exactly these lines for machine-readability:
  SELECTED_TASK_PATH=<relative path>
  SELECTED_TASK_REASON=<one-line reason>
  SELECTED_TASK_VERIFICATION=<primary test command(s)>
- Also write a JSON artifact to {chain_dir}/selected-task.json with fields:
  {"task_path":"...","reason":"...","verification":"..."}

If no viable task exists, set SELECTED_TASK_PATH=NONE and explain why.

## gemini-coder

Heartbeat Implement phase.

Input:
- Previous step output includes SELECTED_TASK_PATH.
- Artifact available at {chain_dir}/selected-task.json.

Requirements:
1) If SELECTED_TASK_PATH=NONE, do not modify code; report no-op.
2) Otherwise implement the selected task end-to-end with minimal coherent scope.
3) Follow task acceptance criteria and existing repo conventions.
4) Add/update automated tests.
5) Run verification commands (at minimum cargo test; plus task-specific commands).
6) Refactor if needed for clarity and maintainability.
7) Commit when tests pass using conventional commit format.

Output requirements:
- Summary of files changed.
- Exact verification commands run + pass/fail.
- Commit hash (or NO_COMMIT with reason).
- Include line: IMPLEMENTATION_STATUS=success|failed|noop

## reviewer

Heartbeat Fresh-Eyes phase.

Act as an independent verifier with permission to fix issues.

Input:
- Use previous output and {chain_dir}/selected-task.json to understand scope.

Process:
1) Re-run verification from scratch (do not trust previous claims).
2) Perform fresh-eyes review for bugs, regressions, missing edge cases, style/clarity issues.
3) If issues are found, refactor/fix them directly.
4) Re-run tests after each fix until green.
5) If fixes are made and tests pass, create a follow-up conventional commit.

Output requirements:
- Verification report with commands + outcomes.
- List of bugs/issues found and resolved.
- Final verdict.
- Include line: FRESH_EYES_STATUS=pass|fail|noop
