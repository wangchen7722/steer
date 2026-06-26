---
name: steer-creator
description: Author, debug, and customize steer workflows — declarative `.steer` files and their `.j2.md` templates that drive an external coding agent. Use when the user wants to write, modify, review, or debug a steer workflow, customize templates, or asks about steer syntax and mechanisms.
---

Author and debug steer workflows.

steer is a workflow interpreter that drives an external coding agent through
declarative, verifiable workflows. This skill helps you **write** those
workflows: the `.steer` scripts and their `.j2.md` templates.

## Input

The user provides (or you derive):
- what the workflow should accomplish (goal, steps, conditions)
- a workflow name (becomes the `.steer` file under `.steer/workflows/`)

If the goal is unclear, ask the user with the AskUserQuestion tool.

## Steps

1. **Understand the goal.** Use `AskUserQuestion` iteratively until you can
   express the workflow as complete, rigorous program logic. Clarify all
   requirements with the user: iteration counts, success criteria, failure
   handling, artifacts to produce, and any constraints. Do not draft until the
   goal is fully specified.

2. **Choose the right nodes.** The built-in action nodes are:

   | Node | When to use |
   |------|-------------|
   | `task` | Agent does work (code, edits, writing) |
   | `collect` | Agent investigates and reports a finding |
   | `command` | Run a shell command and capture output |
   | `ask` | Get a value from the human user |
   | `judge` | Boolean yes/no decision (used in conditions) |
   | `print` | Show a message, no value produced |

   Each node renders through a `.j2.md` template. You can customize templates
   under `.steer/templates/` and switch sets at runtime with `@template`.

3. **Draft the workflow.** Write the `.steer` file following the syntax in
   `references/syntax.md`. Use control flow (`if`, `loop`, `for`, `func`) to
   express the logic. Add `check=` arguments for steps that need verification.

4. **Validate:**

   ```bash
   steer workflow validate <workflow>
   ```

5. **Simulate** (dry-run to preview every rendered instruction):

   ```bash
   steer workflow simulate <workflow>
   ```

6. **Iterate** until the user is satisfied with the workflow design.

## Key Design Principles

- steer is a **control unit**, not an executor: it advances step by step and
  hands each step to the external agent. The agent does all the real work;
  steer only decides what comes next.
- Each action node becomes **one instruction** to the agent. The agent sees
  only that instruction and the variables you interpolate into it — not the
  steps that come after. Make each instruction self-contained; it can't lean
  on "a later step will check this."
- `check=` turns a step into a **quality gate**: the agent re-runs the step
  until verification passes.
- `judge` is for **decisions**: it records a one-time yes/no into a variable
  (used in `if` / `until`) and does not retry. `check=` is for
  **verification**: it re-runs the step until it passes. Don't reach for
  `judge` when you want the agent to redo work until correct — it won't.
- Templates separate **what to say** (the workflow) from **how to say it** (the
  prompt format). Customize `.j2.md` files under `.steer/templates/` to rephrase
  how a node talks to the agent without touching workflow logic.
- `@context` orients the agent on the whole workflow; `@template` switches the
  prompt style mid-workflow. Set `@context` for anything non-trivial, and reach
  for `@template` when distinct phases need distinct voices.

For the methodology behind these — visibility boundaries, convergence loops,
fan-out, gates, output structure — see `references/best-practices.md`.

## References

Consult these files for detailed syntax, mechanisms, and patterns:

- `references/syntax.md` — Complete steer DSL syntax reference. Read before
  writing or modifying any workflow.
- `references/mechanisms.md` — Template, check, context, and control flow
  mechanisms. Read when you need to understand *how* a feature behaves.
- `references/commands.md` — CLI command reference. Read when you need the
  exact `steer workflow` / `steer instance` flags and outputs.
- `references/best-practices.md` — Workflow design best practices (concept +
  rule + copyable steer examples): instruction/template split, model-visibility
  boundary, sense→judge→act loops, fan-out by partition, gates, summary/print.
  Read when designing any non-trivial multi-step workflow.
- `references/writing.md` — Writing accurate, unambiguous instructions:
  diagnosis checklist, common issues, and patterns per node type. Read while
  drafting instructions.

Read the relevant reference before writing or modifying a workflow. When in
doubt, validate first (`steer workflow validate`) and simulate to preview.
