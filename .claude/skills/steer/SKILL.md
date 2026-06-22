---
name: steer
description: Drive a steer (.steer) workflow to completion. Use when the user wants to run a steer workflow — step it one instruction at a time, execute each instruction, report the result back via `steer instance set`, and advance with `steer instance check` until the run is complete.
license: MIT
---

# steer — driving a workflow

`steer` is an external control unit (a "PC"): it holds a workflow's program
counter and hands you **one instruction at a time**. You execute each
instruction, report the result back, and steer advances. This skill drives that
loop.

steer never runs shell or touches files itself — **you** do the work each
instruction describes, and you report results back through `steer instance set` /
`steer instance check`.

## Inputs

The user gives (or you derive):
- a workflow file path (`.steer`), e.g. `.steer/workflows/smoke-bugfix.steer`
- an **instance name**, e.g. the bug id (`login-500`) or a short label

If either is unclear, ask the user with the AskUserQuestion tool.

## Authoring / checking a workflow first (optional)

Before running, you may check it is well-formed:

```bash
steer workflow validate <workflow>      # syntax + semantic checks
steer workflow simulate <workflow>      # print every instruction it would emit
```

## The run loop

1. Start the instance once:

   ```bash
   steer instance start <workflow> <name>
   ```

2. Repeat until a step prints `(complete)` (or `(not running)`):

   a. **Get the current instruction:**

      ```bash
      steer instance step <name>
      ```

      It prints the rendered instruction (or `(complete)`).

   b. **Do exactly what that instruction says.** Execute only the current
      instruction — do not skip ahead or do later steps. For value-producing
      ops the instruction tells you which variable to set and in what format.

   c. **Report the result back** (this is how the agent returns data to steer):

      - For a **value op** (the instruction has a `<report>` tag):

        ```bash
        steer instance set <name> <var> <value>
        ```

        `<value>` is a typed JSON literal: `[1,2,3]` (list), `42` (int),
        `true`/`false` (bool), `"text"` (string), or a bare word treated as a
        string.

      - For a **task with a `check`**: perform the check it describes, then:

        ```bash
        steer instance set <name> checked true   # or false
        ```

   d. **Advance:**

      ```bash
      steer instance check <name>
      ```

      Possible results:
      - `advanced` — the op passed; loop back to `step`.
      - `pending` — you haven't reported the value/flag yet; do step (c) then
        `check` again.
      - `failed` — the check failed; re-read the instruction, fix, and retry
        (set the value / `checked` again, then `check`).

3. If something is unrecoverable, halt the run:

   ```bash
   steer instance error <name> "<reason>"
   ```

4. At any time you can inspect state:

   ```bash
   steer instance status <name>     # running (pc=N) / complete / halted: <reason>
   ```

## How to read an instruction

Each instruction is an XML block whose root tag names the node type (`task`,
`ask`, `command`, `collect`, `judge`, `print`). The tags tell you what to do
and the exact command to report back:

- `<instruction>` — the one unit of work to do (natural language).
- `<report>` — when present, the exact command to report a value, already
  filled with the instance name and target, e.g.
  `steer instance set <name> <var> <value>`, plus a format hint. Do the work,
  then run that command with a value matching the format.
- `<produce>` — files the op should create.
- `<answer>` (judge) — answer `true`/`false` via the given command.

`<name>` is this run's instance name; `<value>` is what you fill in.

## Rules

- Execute **only** the current instruction. Never look ahead or do later steps.
- Always `set` the value (or `checked`) before `check`, or `check` returns
  `pending`.
- On `failed`, fix and retry the **same** instruction — do not skip it.
- `command("...")` means run that shell command and set its result.
  `ask("...")` means ask the user (AskUserQuestion) and set their answer.

## Output

When the run is `complete`, summarise what the workflow accomplished and where
any artifacts were written. If it was `halted`, report the reason.
