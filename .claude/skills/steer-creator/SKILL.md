---
name: steer-creator
description: Author and debug steer workflows. Use when the user wants to write, modify, or review a .steer workflow file, create templates, or asks about steer syntax and mechanisms.
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

- steer is a **control unit**: it holds the program counter, never executes.
  All work is done by the external agent.
- Each action node renders into one instruction for the agent. The agent
  executes it, reports the result, and steer advances.
- `check=` enables **unbounded retry**: the agent re-executes the same
  instruction until verification passes. Use `loop ... until` with a counter
  for bounded retries.
- Templates separate **what to say** from **how to say it**. Customize `.j2.md`
  files under `.steer/templates/` to change the prompt format without touching
  the workflow logic.
- `@context` gives the agent a high-level description of what the workflow does.
  Always set it for non-trivial workflows.
- `@template` switches the active template directory at runtime, useful for
  multi-phase workflows with distinct prompt styles.

## References

Consult these files for detailed syntax, mechanisms, and patterns:

- `references/syntax.md`: Complete steer DSL syntax reference
- `references/mechanisms.md`: Template, check, context, and control flow mechanisms
- `references/commands.md`: CLI command reference
- `references/instruction-writing.md`: Writing accurate, unambiguous instructions
- `references/best-practices.md`: Patterns and anti-patterns for workflow authoring

Read the relevant reference before writing or modifying a workflow. When in
doubt, validate first (`steer workflow validate`) and simulate to preview.
