# OpenSpec Apply Workflow

## Purpose

The `openspec-apply` `.steer` workflow drives the back half of a spec-driven
change — it executes the plan produced by the propose phase, updates task
checkboxes, and then verifies the change via `openspec-verify-change` against a
hardcoded seven-check list before the change can be archived.

## Requirements

### Requirement: The workflow reuses the propose template and requires a matching change directory

The workflow SHALL declare `@template = "openspec-superpowers"` (shared with the
propose workflow), a `@context` stating it "requires the propose workflow to have
run first", and a `@description` surfaced to `steer workflow list`. It SHALL ask
exactly one input variable — `change` — with a kebab-case `return=` type, and
that slug MUST match an existing `openspec/changes/<change>/` directory produced
by the propose workflow so this workflow reads the same artifact set.
Evidenced by the `@template`/`@context`/`@description` directives and the
`change = ask(...)` call in `.steer/workflows/openspec-apply.steer`.

#### Scenario: the change slug must match an existing propose directory

- **WHEN** the workflow runs
- **THEN** it asks the `change` slug once with a kebab-case `return=` type whose
  prompt requires an existing `openspec/changes/<change>/` directory produced by
  the propose workflow.

#### Scenario: the template and context declare the propose dependency

- **WHEN** the workflow module is loaded
- **THEN** `@template` is `"openspec-superpowers"`, `@context` declares the
  dependency on the propose workflow having run first, and `@description` is the
  value shown by `steer workflow list`.

### Requirement: Four sequential skill-availability gates halt before any work begins

At startup the workflow SHALL run four sequential skill-availability gates, each
its own `judge` + `if not` + `print` + `return` block, checking in order
`superpowers:subagent-driven-development`, `superpowers:test-driven-development`,
`superpowers:requesting-code-review`, and `openspec-verify-change`. If any one is
absent the workflow SHALL print a STOP message naming that exact skill and return
before doing any work. Evidenced by the four `has_*` / `if not` / `return` blocks
in `.steer/workflows/openspec-apply.steer`.

#### Scenario: a missing apply-phase skill halts before any work

- **WHEN** any of the four required skills is absent from the available skills
  list
- **THEN** the workflow prints a STOP message naming that one missing skill and
  returns before the apply or verify phase runs.

#### Scenario: all four gates pass before work begins

- **WHEN** all four skills are present
- **THEN** the workflow proceeds past the gate block to the apply phase.

### Requirement: The apply phase tool-checks the plan then executes micro-tasks updating task checkboxes

The `apply_change` phase SHALL first verify each CLI tool named in `plan.md` via
`which <tool>`, asking the user about any missing tool before proceeding, and
then execute the micro-tasks in `plan.md`, updating `tasks.md` checkboxes from
`- [ ]` to `- [x]` as coarse tasks complete. Its `check=` SHALL require every
`plan.md` CLI tool to be available or explicitly user-confirmed, every
`tasks.md` checkbox to be `- [x]`, and any spec changes triggered during
implementation to be reflected in `specs/`. Evidenced by the `apply_change(...)`
call and its `check=` text in `.steer/workflows/openspec-apply.steer`.

#### Scenario: missing plan tools are surfaced to the user before execution

- **WHEN** a CLI tool named in `plan.md` is not found by `which`
- **THEN** the apply phase asks the user about that tool before executing the
  plan's micro-tasks.

#### Scenario: task checkboxes close as coarse tasks complete

- **WHEN** a coarse task in `tasks.md` is completed during plan execution
- **THEN** its checkbox is updated from `- [ ]` to `- [x]` and the apply phase's
  `check=` requires no `- [ ]` gap to remain.

### Requirement: The verify phase produces verify.md with the hardcoded seven-check list

The `verify` phase SHALL run `openspec-verify-change` against
`openspec/changes/{change}/` and produce `verify.md` documenting all seven checks
— Spec Coverage, Implementation Coverage, Scenario Testability, Breaking Changes,
Behavioral Alignment, Front-Door Routing Leak Detector, and Template Comment
Stray Check — with an Overall Decision. Its `check=` SHALL require `verify.md` to
exist, all seven checks to be documented, the Overall Decision to be PASS, and no
`- [ ]` gap to remain. The seven-check enumeration is authoritative in
`.steer/templates/openspec-superpowers/verify.j2.md`. Evidenced by the
`verify(...)` call and its `check=` in `.steer/workflows/openspec-apply.steer`.

#### Scenario: verify.md covers the fixed seven checks

- **WHEN** the `verify` phase runs
- **THEN** it produces `verify.md` documenting all seven checks (Spec Coverage,
  Implementation Coverage, Scenario Testability, Breaking Changes, Behavioral
  Alignment, Front-Door Routing Leak Detector, Template Comment Stray Check) and
  an Overall Decision.

#### Scenario: a remaining checkbox gap fails the verify check

- **WHEN** `verify.md` records any unresolved `- [ ]` gap or a non-PASS Overall
  Decision
- **THEN** the `verify` phase's `check=` fails because no `- [ ]` gap may remain.

### Requirement: The archive precondition requires all checkboxes closed and changes committed

The verify phase's `check=` SHALL require that every `tasks.md` checkbox is
`- [x]` and all code changes are committed (no unstaged files), establishing the
archive precondition: the change is ready to archive only when both hold.
Evidenced by the `verify(...)` `check=` text and the `verify.j2.md` template's
gap-tracking rules in `.steer/templates/openspec-superpowers/verify.j2.md`.

#### Scenario: uncommitted changes block archive

- **WHEN** code changes are left unstaged when the verify phase runs
- **THEN** the archive precondition is not met because the verify `check=`
  requires all changes to be committed.

#### Scenario: the closing print names the archive readiness gate

- **WHEN** the verify phase completes
- **THEN** the workflow prints an instruction to review `verify.md` and states
  that if PASS the change is ready to archive.
