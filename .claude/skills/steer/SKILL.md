---
name: steer
description: Drive a steer workflow to completion. Use when the user wants to run a steer workflow, validate or simulate a workflow, or mentions `steer instance` / `steer workflow` commands.
---

Run a steer workflow to completion.

steer is a workflow interpreter: it parses a `.steer` workflow into individual instructions and hands them to you one at a time. You execute each instruction, report the result back, and steer advances to the next step.

## Input

You need two things; resolve either with **AskUserQuestion** if missing.

- a **workflow** — a path to a `.steer` file (e.g. `./my.steer`) or a bare
  name (e.g. `demo`, `openspec-propose`). `steer instance start` resolves a bare
  name under `.steer/workflows/` itself, so don't look for the file — only a
  path needs an existence check.
- an **instance name** — a short label for this run. **Always confirm it with
  `AskUserQuestion`**: suggest a kebab-case default derived from the workflow
  name and the user's request, but let the user pick or override it. Do not
  invent a name silently. 

If the user didn't name a workflow, discover candidates:

```bash
steer workflow list
```

This prints each workflow name with its `@description`. It never errors on
a bad/empty directory, so an empty list means there are no workflows here.

Match the user's request against those descriptions, then confirm the choice
with **AskUserQuestion** — recommend the best fit (the user can always pick
"Other" to name one you didn't suggest).

## Steps

1. **Start the instance**

   ```bash
   steer instance start <workflow> <name>
   ```

   `<workflow>` may be a path or a bare workflow name (resolved under
   `.steer/workflows/`). Output: `instance <name>: started`, followed by the
   workflow context description if `@context` was set in the workflow.

2. **Loop** until `step` returns `(complete)` or `(not running)`:

   a. **Get the current instruction:**

      ```bash
      steer instance step <name>
      ```

   b. **Execute the instruction.** Read it and follow exactly what it says.

   c. **Report the result back** (when the instruction tells you to set a value):

      ```bash
      steer instance set <name> <var> <value>
      ```

      `<value>` is a typed JSON literal: `[1,2,3]` (list), `42` (int), `true`/`false` (bool), `"text"` (string), or a bare word treated as a string.

   d. **Check and advance:**

      ```bash
      steer instance check <name>
      ```

      Possible results:
      - `advanced` — op passed. Call `step` to get the next instruction.
      - `pending` — value not reported yet. Do step (c) then `check` again.
      - an **instruction** — steer is asking you to verify the work. Perform the verification, report via `set checked`, then `check` again.
      - `failed` — check failed. Call `step` again to re-read the same instruction with the failure context, fix, and retry.

3. **If unrecoverable**, halt:

   ```bash
   steer instance error <name> "<reason>"
   ```

4. **Inspect state** at any time:

   ```bash
   steer instance status <name>     # running / complete / halted: <reason>
   ```

   The output includes the workflow context (if `@context` was set) and the current run status.

## Guardrails

- Always `set` the value (or `checked`) **before** `check`, or `check` returns `pending`.
- On `failed`, retry the **same** instruction. Do **not** skip it.
- If a check fails repeatedly (retry count ≥ 10), use `steer instance error` to halt instead of looping forever.
- If you genuinely cannot reach a confident answer, use `steer instance error` instead of fabricating a result.

## Output

When the run is `complete`, summarise what the workflow accomplished and where any artifacts were written. If it was `halted`, report the reason.
