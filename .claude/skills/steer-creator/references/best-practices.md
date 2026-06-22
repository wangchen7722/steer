# Steer Workflow Best Practices

## Workflow Design

### Start with `@context`

Always set `@context` for non-trivial workflows. It gives the agent a
high-level summary of the workflow's purpose, shown on `instance start` and
`instance status`.

```steer
@context = "multi-step code review with automated fix and verification"
```

### Use the Right Node for the Job

| Situation | Use | Why |
|-----------|-----|-----|
| Agent does open-ended work | `task` | Universal primitive, supports `check` and `produce` |
| Need human input | `ask` | Guarantees value comes from the user |
| Run a shell command | `command` | Structured capture of output |
| Agent investigates and reports | `collect` | Emphasizes grounding in evidence |
| Yes/no decision | `judge` | Returns boolean, usable in conditions |
| Show a message | `print` | No value, no verification |

### Write Accurate Instructions

Each instruction is handed to the agent as-is. The agent has no context beyond
what the instruction provides. Accuracy matters more than brevity.

See `references/instruction-writing.md` for a systematic method: diagnosis
checklist, common issues and fixes, and patterns per node type.

Key principles:
- State the **goal** and **constraints** explicitly.
- Interpolate earlier results via `{variables}` so the agent has context.
- Specify the expected output format in `return=`.
- Add `check=` with **concrete, repeatable** verification criteria.
- Avoid vague phrases ("fix it", "make it good", "confirm it works").

### Use `check=` for Quality Gates

Add `check=` to any step where the agent's work must be verified. This enables
the retry mechanism: the agent re-executes until verification passes.

```steer
task("Write a regression test for {bug} that reproduces the failing behavior described in: {root_cause}",
     produce=["tests/{bug}_test.rs"],
     check="Run `cargo test {bug}` and confirm zero failures")
```

Without `check=`, the step advances immediately after the agent reports
completion. Use `check=` for any step where correctness matters.

### Use `produce=` to Declare Artifacts

`produce=` tells the agent what files to create. It appears in the rendered
instruction as a `<produce>` block. This makes the expected output explicit.

```steer
task("Create a proposal document for {change} that identifies goals, non-goals, risks, and the validation plan",
     produce=["docs/proposals/{change}.md"],
     check="Confirm the proposal contains all four sections: goals, non-goals, risks, and validation plan")
```

## Control Flow Patterns

### Bounded Retry with `loop ... until`

Unbounded retry (`check=` alone) can loop forever. Wrap it in a `loop ... until`
with a counter for bounded attempts:

```steer
attempt = 0
passed = false
loop
    attempt = attempt + 1
    task("Apply a fix for {bug} based on root cause: {root_cause}. This is attempt {attempt}; record the change applied and why it should address the root cause. Preserve existing behavior for all other code paths.",
         produce=["artifacts/bugfix-{bug}-attempt-{attempt}.md"],
         check="Run `cargo test {bug}` and confirm zero failures")
    passed = judge("Does `cargo test {bug}` exit with code 0?")
until passed or attempt >= 3

if not passed
    task("Write a handoff document for {bug} containing: root cause, attempted fix, failing evidence, and the next diagnostic step",
         produce=["artifacts/bugfix-{bug}-handoff.md"],
         check="Confirm the handoff includes root cause, evidence, and next steps")
    return
end
```

### Two-Phase: Investigate Then Act

Use `collect` to investigate, then `task` to act on the findings:

```steer
root_cause = collect("Reproduce {bug} by running `cargo test {bug}`, inspect the failing code path in the stack trace, and summarize the root cause. Include the specific function, line range, and the observed vs expected behavior.",
                     return="root cause: function, line range, observed symptom, why it diverges",
                     check="Confirm the summary names the failing function and the observed symptom")

task("Apply the smallest safe fix for {bug} based on root cause: {root_cause}. Preserve existing behavior for all other code paths.",
     produce=["artifacts/bugfix-{bug}.md"],
     check="Run `cargo test {bug}` and confirm zero failures")
```

This separates understanding from execution, producing better results.

### Review Loop with `for`

Iterate over a dynamic list for review:

```steer
files = command("git diff --name-only -- . ':!target'", return="list of changed file paths")

for f in files
    task("Review {f} for accidental broad changes. Only changes directly related to {bug} are acceptable; simplify or revert anything else.",
         check="Confirm {f} contains only changes needed for {bug}")
end
```

### Conditional Logic with `if`

Branch based on earlier results:

```steer
has_tests = command("test -d tests/ && echo yes || echo no", return="yes or no")

if has_tests == "yes"
    task("Run the existing test suite via `cargo test` and report any failures",
         check="All tests pass with zero failures")
else
    task("Create a minimal regression test at tests/{bug}_test.rs that reproduces the failing behavior for {bug}. The test must fail against the current (unfixed) code and pass once the fix is applied. Follow the existing test conventions in the `tests/` directory.",
         produce=["tests/{bug}_test.rs"],
         check="Run `cargo test {bug}` and confirm the test compiles and passes")
end
```

### Reusable Logic with `func`

Extract repeated patterns into functions:

```steer
func review_file(f)
    task("Review {f} for issues: unused imports, dead code, inconsistent naming. Fix only clear problems, not style preferences.",
         check="Confirm {f} has no unused imports or dead code")
end

for f in files
    review_file(f)
end
```

## Template Best Practices

### Prefer Default Templates, Customize When Needed

The default templates under `.steer/templates/default/` are well-tuned. Only
create custom templates when you need a significantly different prompt style.

### Use `on_check` for Domain-Specific Verification

Customize the check instruction in your template's front-matter:

```yaml
on_check: |
  Verify the following:
  <check>{{ check }}</check>
  Inspect the work and confirm the condition holds.
```

This lets you change how verification is phrased per node type, without
modifying the VM.

### Use `@template` for Multi-Phase Workflows

Different workflow phases often need different prompt styles:

```steer
@template = "planning"
task("create proposal", ...)     # concise, structured prompts

@template = "default"
task("implement proposal", ...)  # standard prompts with full context
```

Create `.steer/templates/planning/task.j2.md` with a template that emphasizes
high-level thinking over detailed implementation.

### Template Naming Convention

- Directory name = phase name (e.g. `planning/`, `review/`, `implementation/`).
- File name = node name (e.g. `task.j2.md`, `collect.j2.md`).
- Only override the nodes you need to customize; others fall back to `default/`.

### Inline Output Skeletons into the Template

When a phase must produce a file with a fixed structure (sections, headings,
checklist format), inline that skeleton directly into the phase's `.j2.md`
rather than keeping it in a separate scaffold file the agent has to `Read`
at runtime by path.

Inlining is better for three reasons:

- **`simulate` shows the full skeleton.** With a separate scaffold file, the
  rendered instruction only says "read scaffold at <path>", so you cannot
  review the actual structure in a dry-run. Inlined, `simulate` prints the
  complete skeleton.
- **One fewer Read round-trip** for the agent at runtime.
- **No path coupling.** A separate scaffold and its template are linked only
  by a path string; editing one and forgetting the other silently
  desynchronizes them. An inlined skeleton is self-contained in one file.

Before inlining, confirm the scaffold text has no `{`, `}`, or `%`
characters — they collide with the Jinja2 subset the template engine
supports. A count of zero means the text is safe to inline verbatim:

```bash
grep -cE '[{}%]' <scaffold-file>   # 0 = safe to inline
```

### Wrap Output Skeletons in `<template>`

When a template carries an inlined output skeleton, wrap it in
`<template>...</template>` tags with a one-line preamble. The tags tell the
agent this block is a structure to fill in — not prose to copy verbatim:

```jinja
Use the following as your output template. Follow this structure exactly,
replacing each `<!-- ... -->` placeholder with real content and removing the
placeholder comments from the final file.

<template>
## Why

<!-- Explain the motivation for this change. What problem does this solve? Why now? -->

## What Changes

<!-- Describe what will change. Be specific about new capabilities, modifications, or removals. -->
</template>
```

The `<template>` tags are plain text to the Jinja2 engine — they pass
through unchanged and do not affect parsing. They serve as a semantic
marker the agent reads as "this is the form my output must take."

Keep three concerns separate in a phase template: **rules** (constraints the
agent obeys) go in a `<rules>` block, the **skeleton** (the form the output
takes) goes in a `<template>` block, and the phase-specific dynamic content
goes in `<instruction>`. A template with all three is self-documenting.

### Prefer `judge` Gates Over Textual "CHECK" Prompts

A template that says "CHECK: Ensure skill X is available. If MISSING, STOP."
is a **prompt-level** instruction — it relies on the agent reading and
obeying it. steer is a control unit; gates belong in the control flow, not
the prose. Use `judge` + `if not ... return` to make the gate real: the
workflow actually halts when the precondition fails.

```steer
// BAD: textual gate — depends on the agent obeying "STOP"
task("CHECK: Ensure `superpowers:brainstorming` is in your skills list. If MISSING, STOP.")

// GOOD: programmatic gate — the workflow halts if the skill is absent
has_brainstorming = judge("Is `superpowers:brainstorming` present in your available skills list? Answer true only if you can see it listed; false otherwise.")
if not has_brainstorming
    print("STOP: `superpowers:brainstorming` is required but not in your skills list. Install it and re-run.")
    return
end
brainstorm("...", produce=[...], check="...")
```

Use one `judge` per distinct precondition, so the STOP message names exactly
which one is missing. Place each gate immediately before the phase that
needs it, so a failure is reported where it matters — unless several
preconditions are all required before any work begins, in which case a
single upfront gate block is cleaner than scattering gates across phases.

## Anti-Patterns

### Don't Use `judge` for Verification

`judge` records a one-time answer. It does **not** retry on failure. Use
`check=` on `task`/`collect` when you need the agent to re-do work until it's
correct.

### Don't Nest Calls in Conditions

Calls cannot appear as sub-expressions. Use a two-step pattern:

```steer
// BAD:
if judge("is it fixed?")  // ERROR: call in condition

// GOOD:
fixed = judge("is it fixed?")
if fixed
    ...
end
```

### Don't Skip Validation

Always run `steer workflow validate` before running a workflow. Syntax errors
and semantic violations are caught early, preventing confusing runtime failures.

### Don't Over-Complicate Simple Flows

Not every workflow needs `loop`, `func`, or `if`. A simple linear sequence is
perfectly valid:

```steer
@context = "add linting to the project"
task("Install the `clippy` linter via `rustup component add clippy`", check="`cargo clippy --version` exits with code 0")
task("Create or update `.clippy.toml` with the project's lint configuration", check="The config file exists and `cargo clippy` reads it without errors")
task("Fix all existing clippy warnings. Do not suppress warnings with #[allow]; fix the underlying code.", check="`cargo clippy` reports zero warnings")
print("linting setup complete")
```

### Don't Use `command` for Agent Work

`command` runs a literal shell command. Don't use it when you need the agent to
do open-ended work. Use `task` or `collect` instead.

```steer
// BAD: using command for agent reasoning
result = command("think about the root cause", return="root cause")

// GOOD: using collect for agent reasoning
result = collect("Reproduce {bug} by running `cargo test {bug}`, inspect the failing code path in the stack trace, and summarize the root cause. Include the specific function, line range, and the observed vs expected behavior.",
                 return="root cause: function, line range, observed symptom, why it diverges")
```

### Don't Set `checked` Without a Reason on Failure

When a check fails, the `reason` field is required and must be non-empty.
Bare `checked false` is rejected by the VM.

```bash
# BAD:
steer instance set myrun checked false              # ERROR

# GOOD:
steer instance set myrun checked '{"passed":false,"reason":"test still failing"}'
```

## Testing and Debugging

### Use `simulate` to Preview

Before running a workflow, simulate it to see every rendered instruction:

```bash
steer workflow simulate my-workflow
```

This walks the entire workflow and prints what the agent would see, including
both branches of `if` and all function bodies.

### Validate After Every Change

Make validation part of your edit cycle:

```bash
steer workflow validate my-workflow
```

Catch errors early. A typo in a `check=` string or a missing `return=` on
`ask` will fail at runtime otherwise.

### Instance Storage

Instances live under `.steer/instances/<name>/`:

```
.steer/instances/<name>/
├── context.json    # full execution state (resumable)
└── audit.jsonl     # audit trail
```

Starting an instance with an existing name clears and recreates it. There is no
history mechanism (v1).

### Resume After Interruption

Because the full context is serialized to JSON, instances can be resumed across
CLI invocations. If the agent crashes or the session ends, start a new session
and call `steer instance step <name>` to continue where you left off.
