---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the following:
  <check>{{ check }}</check>
  Inspect the work and confirm the condition holds.
---
This is a Superpowers-powered spec-driven workflow. Current position: apply — Step 1 (execute).

If you have the Agent tool available (or can spawn new subagent sessions), invoke `superpowers:subagent-driven-development` via the Skill tool. It will execute `plan.md` (in this change directory) micro-tasks with fresh subagents per task.

Tell the executor:
- Read `plan.md` in this change directory for micro-tasks
- Update `tasks.md` checkboxes as coarse tasks complete

If you do NOT have the Agent tool available, invoke `superpowers:test-driven-development` and `superpowers:requesting-code-review` directly. Complete tasks one by one in sequence:
1. Invoke test-driven-development for each task in plan.md
2. After each task, invoke requesting-code-review
3. Update tasks.md checkboxes as tasks complete

<instruction>{{ instruction }}</instruction>
<rules>
- LANGUAGE: Write all output in English, regardless of the user's language. Code comments and variable names follow the project's existing conventions, but prose MUST be English.
- Spec sync rule: If code review or testing reveals new requirements or changes to existing requirements, you MUST update the corresponding spec files in `specs/` before continuing implementation. Do NOT implement behavior that lacks a spec basis without first writing or updating the spec. This workflow is spec-driven: specs lead, code follows.
- Execute only this instruction. Do NOT skip ahead or do unplanned work.
</rules>
