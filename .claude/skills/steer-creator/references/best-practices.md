# Steer Workflow Best Practices

How to design steer workflows well — concept + rule + a copyable steer example
for each. This is the design methodology made concrete: the rules are stated,
then shown in code you can adapt. For the tactical side of phrasing a single
instruction (diagnosis checklist, per-node patterns), see `writing.md`; for
DSL syntax see `syntax.md`; for template/check/context mechanisms see
`mechanisms.md`.

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

See `references/writing.md` for a systematic method: diagnosis
checklist, common issues and fixes, and patterns per node type.

Key principles:
- State the **goal** and **constraints** explicitly.
- Interpolate earlier results via `{variables}` so the agent has context.
- Specify the expected output format in `return=`.
- Add `check=` with **concrete, repeatable** verification criteria.
- Avoid vague phrases ("fix it", "make it good", "confirm it works").

### The Instruction Positions; the Template Body Executes

A step has two surfaces the model sees: the **instruction** (the workflow's
positional argument) and the **template body** (the `.j2.md` the step
expands). The instruction is one positioning sentence — what this step does,
what to read first, the handoff. The template body carries the mechanism, the
criteria, and the why. Keep their jobs separate: when both explain the
mechanism, the model has to reconcile two statements of the same thing, and
when they drift the model picks one and is wrong about the other.

A `<rules>` block in a template earns its place only by adding *why-mandatory*
(intent) or *do-not* (a failure mode the placeholder cannot express). The
deletion test: *could the model infer this rule from the body or a placeholder
alone?* If yes, it is duplication — cut it.

```steer
// Instruction positions in one sentence; "how" lives in the template body.
task("Draft the proposal for {change}: read brainstorm.md first, then write proposal.md with Why / What Changes / Capabilities / Impact.",
     produce=["openspec/changes/{change}/proposal.md"],
     check="Confirm proposal.md has Why, What Changes, Capabilities, and Impact sections")
```

### Hold the Model-Visibility Boundary

The model executing a step sees only the instruction and the template — not
the workflow's control flow, the origin of interpolated variables, or the
relationship between steps. A variable like `{covered}` interpolates to a
literal `covered=true` / `covered=false` in the text the model reads; the model
sees the value, not "this came from the judge step before the final refine."
Describe what a value *means for what the model must write*, never its origin.
Do not narrate step correspondence in template prose unless the model can
observe both sides — if the correspondence matters, enforce it in `check=`.
Reserve cross-step rationale (why a gate runs once, why a loop is post-test)
for the workflow's own comments.

```steer
// GOOD: the instruction tells the model what covered=true means for its work.
task("Spec coverage is {covered}. If covered is false, write the missing spec sections named in the gap list; if true, just confirm no regressions.",
     check="Confirm every gap-listed capability has a spec file")

// BAD: the instruction explains provenance the model cannot act on.
task("The judge step before this one set covered. Now match the coverage_guard step's verdict and reconcile.",
     check="...")
```

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

### Run Convergence Work as Sense → Judge → Act, Post-Test

Iterative-convergence work (review, coverage, refinement) is best structured
as a loop of three distinct roles, not one monolithic step: **Sense** reads
the world and records state (PASS / GAP) without fixing anything; **Judge**
is a pure boolean over the sensed state (is the GAP empty?); **Act** mutates
the world to close the reported gap. Separating the roles lets the loop
condition rest on the judge, lets sensing fan out by partition, and keeps the
act step focused on a concrete delta. Make the loop post-test, and guard the
act step inside the body so a round that is already covered does not refine.

```steer
round = 0
loop
    gaps = collect("Read the specs in {dir} and list capabilities with no spec file. Report the gap list; if none, report empty.",
                   return="gap list (capability names), empty if none",
                   check="Confirm the list names only capabilities missing a spec")
    covered = judge("Is the gap list empty? Answer true only if no capabilities are missing a spec.")
    if not covered
        task("Write the missing spec files for these capabilities: {gaps}. One file per capability.",
             produce=["specs/{gaps}.md"],
             check="Confirm one spec file exists per gap-listed capability")
    end
    round = round + 1
until covered or round >= 5
```

Note `covered` here reflects the judge *before* the final act — see the
anti-pattern on stale loop variables below for how to report it honestly.

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

### Concurrency: Fan Out vs. Sequential `for`

Reach for the right shape when work repeats over a list:

- **Sequential `for`** when each item depends on the last, or when order
  matters, or when the list is short enough that doing them one at a time is
  fine.
- **Fan out** (multiple agents running concurrently) when the items are
  independent and there are enough of them that parallelism pays off.

```steer
// Sequential: review each changed file in turn.
files = command("git diff --name-only -- . ':!target'", return="changed file paths")

for f in files
    task("Review {f} for accidental broad changes. Only changes directly related to {bug} are acceptable; simplify or revert anything else.",
         check="Confirm {f} contains only changes needed for {bug}")
end
```

When fanning out to concurrent agents that all write to the same shared file,
have each agent return its result and let the main agent merge and append —
don't let two agents append to the same file at once, or entries get lost.

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

### Summary Is the Record; Print Is the Headline + Pointer

When a workflow produces both a summary artifact and a terminal print, make
the summary the comprehensive record and the print a brief condensation that
points to it. The summary is the handoff artifact (the whole task, every
intermediate file with a one-line meaning, the final results); the print is a
few facts (run name, outcome, where to look) plus a pointer. The print does
not re-enumerate files and does not branch on the outcome — branch the
*summary* instead, and let the print interpolate the outcome into one line.
Boundary: if a detail belongs in the summary, it does not belong in the print.

```steer
// Summary branches on outcome; print just interpolates the headline.
if fixed
    task("Write the final report: root cause, the fix applied, the regression test added, and the verification run. One line per artifact produced.",
         produce=["artifacts/bugfix-{bug}.md"],
         check="Confirm the report names the root cause, fix, test, and verification")
    print("bugfix-{bug}: fixed. Full report in artifacts/bugfix-{bug}.md.")
else
    task("Write a handoff: root cause, attempts made, failing evidence, next diagnostic step.",
         produce=["artifacts/bugfix-{bug}-handoff.md"],
         check="Confirm the handoff has root cause, attempts, evidence, next step")
    print("bugfix-{bug}: not fixed in {attempt} attempts. Handoff in artifacts/bugfix-{bug}-handoff.md.")
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
changing steer's behavior. The `check=` argument is the single source of check
criteria; `on_check` only frames it — do not duplicate the check text into
`on_check`.

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
correct. `judge` is for *is this precondition met?* (gate); `check=` is for
*did the work succeed?* (verify-and-retry).

### Don't Treat a Stale Loop Variable as Final Truth

At the end of a sense→judge→act loop, the loop-control variable reflects the
judge **before** the final act — the act ran but was not re-sensed, so a
`covered = false` may be stale: the final act may have closed the gap. Do not
assert a residual gap as fact ("N units still uncovered") when it was measured
before the last fix; do not paper it over as resolved either. Report the
provenance ("last review found these gaps; refine ran after it and was not
re-confirmed") and hand the residual list to the user as a checklist.

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
Bare `checked false` is rejected by steer.

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
