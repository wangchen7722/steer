# Writing Effective Steer Instructions

Every action node's first positional argument becomes the `instruction` that the
agent reads and follows. The quality of the workflow's output depends directly
on the quality of these instructions. This document provides a systematic
method for writing accurate, unambiguous instructions.

## Why Instruction Quality Matters

Each instruction is handed to the agent as-is, rendered through a template.
The agent has **no context beyond what the instruction provides** (plus any
retry context on failure). Vague or ambiguous instructions produce vague or
wrong results; precise instructions produce precise results.

## Instruction Diagnosis Checklist

Before finalizing any instruction, check it against these dimensions. If any
item is missing or unclear, revise the instruction before proceeding.

### Completeness

- [ ] **Goal**: Does the instruction state exactly what the agent should do?
- [ ] **Scope**: Is it clear what is in scope and what is out of scope?
- [ ] **Input**: Are all necessary inputs referenced (via variables, context)?
- [ ] **Output**: Is the expected output format or content described?
- [ ] **Constraints**: Are there constraints the agent must respect (no extra
  files, minimal changes, follow existing patterns)?
- [ ] **Produce**: When `produce=` is set, does the instruction describe what
  each produced file should contain and its purpose?

### Specificity

- [ ] **No ambiguity**: Could a reasonable agent interpret this instruction in
  more than one way? If so, disambiguate.
- [ ] **No filler**: Remove hedging words ("maybe", "if possible", "try to").
  Either the agent should do the thing or it should not.
- [ ] **Concrete criteria**: Instead of "make it good", specify what "good"
  means (e.g. "all tests pass", "no compiler warnings", "follows the pattern
  in `src/example.rs`").

### Context Grounding

- [ ] **References**: Does the instruction point to specific files, patterns,
  or examples the agent should follow?
- [ ] **Variables**: Are earlier workflow results interpolated via `{var}` so
  the agent has the data it needs?
- [ ] **Why**: For non-obvious tasks, does the instruction explain *why* the
  work matters (e.g. "so the CI pipeline can validate the change")?

## Common Issues and Fixes

| Issue | Impact | Fix |
|-------|--------|-----|
| Too vague ("fix it") | Agent guesses, often wrong | State exactly what to fix and how to verify |
| Missing output format | Agent returns inconsistent data | Specify `return=` format explicitly |
| No scope boundary | Agent does unplanned work | State what NOT to do, or use `produce=` to constrain |
| `produce=` without description | Agent creates files with wrong content | Describe each file's purpose and expected content in the instruction |
| No verification | Wrong results pass silently | Add `check=` with concrete pass criteria |
| Over-specified ("edit line 42...") | Breaks on any code change | Describe the *intent*, not the *mechanics* |
| Missing context | Agent cannot connect to earlier work | Interpolate `{variables}` from prior steps |
| Ambiguous pronoun ("fix that") | Agent misidentifies the target | Use explicit names and variable interpolation |

## Instruction Patterns by Node Type

### `task` Instructions

`task` instructions should describe **what to accomplish**, not how to
accomplish it step-by-step (unless the steps are the requirement). The agent
decides the mechanics.

When `produce=` is set, the instruction should describe what each file should
contain. The template renders a `<produce>` block listing the file paths, but
the instruction text must explain the content and purpose of those files so the
agent writes the right thing.

**Structure**: goal + context + constraints + expected output (+ produce description)

```steer
// Vague:
task("fix the bug")

// Accurate:
task("Fix the {bug} by applying the smallest safe change that addresses the root cause: {root_cause}. Preserve existing behavior for all other code paths. The fix must pass the regression test in tests/{bug}_test.rs.",
     produce=["artifacts/bugfix-{bug}.md"],
     check="Run `cargo test {bug}` and confirm all tests pass with zero failures")
```

### `collect` Instructions

`collect` instructions should specify **what to investigate** and **what to
report back**. The agent must do actual work, not guess.

**Structure**: investigation target + evidence method + report format

```steer
// Vague:
collect("find the root cause", return="root cause")

// Accurate:
collect("Reproduce {bug} by running `make test-{bug}`, inspect the failing code path in the stack trace, and summarize the root cause. Include the specific function, line range, and the observed vs expected behavior.",
        return="root cause: function name, line range, observed symptom, and why it diverges from expected behavior",
        check="confirm the summary names the failing function and the observed symptom")
```

### `command` Instructions

`command` instructions are literal shell commands. Be exact with flags and
paths.

```steer
// Vague:
command("check what changed", return="files")

// Accurate:
command("git diff --name-only -- . ':!target' ':!node_modules'",
        return="list of changed file paths, one per line, excluding build artifacts")
```

### `ask` Instructions

`ask` instructions are the question text shown to the user. Frame it so the
user knows exactly what to provide.

```steer
// Vague:
ask("what bug?", return="bug")

// Accurate:
ask("Which bug ID or issue title should be fixed? Provide the Jira key (e.g. PROJ-123) or a short descriptive title.",
    return="bug identifier (Jira key or short title)")
```

### `judge` Instructions

`judge` instructions must have a **deterministic yes/no answer**. Avoid
questions that require explanation or nuance.

```steer
// Vague:
judge("is it okay?")

// Accurate:
judge("Does `cargo test -- {test_name}` exit with code 0?")
```

### `check` Instructions

`check` instructions (the `check=` argument) tell the agent **how to verify**
the work. They should describe a concrete, repeatable verification step.

```steer
// Vague:
check="confirm it works"

// Accurate:
check="Run `cargo test {bug}` and confirm zero failures. If any test fails, report which test and the error message."
```

### `produce` and Instructions

When a step has `produce=["file1", "file2"]`, the template automatically renders
a `<produce>` block listing the file paths. However, the template only says
"Write or update the following files" with the paths. It does **not** explain
what goes in each file. That explanation must come from the instruction text.

The instruction should describe:
- **What** each produced file should contain (structure, sections, key points).
- **Why** the file exists (purpose in the workflow).
- **Format** if applicable (Markdown, TOML, specific schema).

```steer
// produce without description:
task("create the handoff",
     produce=["artifacts/handoff.md"],
     check="confirm the handoff includes evidence")

// produce with description in instruction:
task("Create a handoff document at artifacts/handoff.md containing four sections: 1) Root cause summary, 2) Attempted fix and why it failed, 3) Failing evidence (test output or error log), 4) Next diagnostic step for the next engineer",
     produce=["artifacts/handoff.md"],
     check="Confirm the handoff contains all four sections with non-empty content")
```

This applies to all node types that support `produce=`: `task`, `collect`,
`command`, `ask`, `judge`, `print`.

## Revision Workflow

When writing instructions for a workflow:

1. **Draft** each instruction as a rough description of the goal.
2. **Diagnose** against the checklist above. Identify missing dimensions.
3. **Clarify** with the user (via `AskUserQuestion`) for any dimension you
   cannot fill yourself: acceptance criteria, scope boundaries, output format,
   constraints.
4. **Rewrite** the instruction incorporating all dimensions.
5. **Simulate** (`steer workflow simulate`) to preview what the agent will see.
   Read the rendered instruction as if you are the agent: is it unambiguous?
6. **Iterate** until every instruction passes the checklist.

This mirrors the prompt-optimizer's analysis pipeline, adapted for steer's
instruction context: each instruction is a self-contained prompt to the agent,
and must carry all the context the agent needs to execute correctly.
