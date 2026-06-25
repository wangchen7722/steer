# OpenSpec Propose Workflow

## Purpose

The `openspec-propose` `.steer` workflow drives the front half of a spec-driven
change — brainstorm, proposal, specs, design, tasks, plan — archiving each
phase's artifact under `openspec/changes/<change>/` and then pausing for human
review before the apply phase runs.

## Requirements

### Requirement: The workflow declares the OpenSpec+Superpowers template and a single input variable

The workflow SHALL declare `@template = "openspec-superpowers"`, a `@context`
that positions it as the propose phase which "pauses for human review before
apply", and a `@description` surfaced to `steer workflow list`. It SHALL ask
exactly one input variable — `change` — with a `return=` type that coerces the
answer to kebab-case, and that single slug SHALL be interpolated into every
phase's `produce=` path so all artifacts land under
`openspec/changes/<change>/`. Evidenced by the `@template`/`@context`/`@description`
directives and the `change = ask(...)` call in `.steer/workflows/openspec-propose.steer`.

#### Scenario: the change slug is asked once and drives all paths

- **WHEN** the workflow runs
- **THEN** it asks the `change` slug once with a kebab-case `return=` type and
  interpolates `{change}` into every phase's `produce=` path under
  `openspec/changes/`.

#### Scenario: the template and context directives are fixed

- **WHEN** the workflow module is loaded
- **THEN** `@template` is `"openspec-superpowers"`, `@context` declares the
  propose phase pauses for human review before apply, and `@description` is the
  value shown by `steer workflow list`.

### Requirement: A skill-availability gate halts before the brainstorm and plan phases

Before the `brainstorm` phase the workflow SHALL `judge` whether
`superpowers:brainstorming` is present in the available skills list and, if not,
`print` a STOP message naming that exact skill and `return` without running the
phase. Before the `plan` phase it SHALL apply the same gate to
`superpowers:writing-plans`. Each gate is its own `judge` + `if not` + `print` +
`return` block so the STOP message identifies the one missing skill.
Evidenced by the two `has_*` / `if not` / `return` blocks in
`.steer/workflows/openspec-propose.steer`.

#### Scenario: a missing brainstorming skill halts before brainstorm

- **WHEN** `superpowers:brainstorming` is not in the available skills list
- **THEN** the workflow prints a STOP message naming `superpowers:brainstorming`
  and returns before the brainstorm phase runs.

#### Scenario: a missing writing-plans skill halts before plan

- **WHEN** `superpowers:writing-plans` is not in the available skills list
- **THEN** the workflow prints a STOP message naming `superpowers:writing-plans`
  and returns before the plan phase runs.

### Requirement: The phase sequence runs brainstorm through plan, each with a per-phase output contract

The workflow SHALL run the phases in source order — `brainstorm`, `proposal`,
`specs`, `design`, `tasks`, `plan` — where each phase is a custom callee with a
`produce=` path list and a `check=` condition, and each phase builds on what the
previous one persisted. The per-phase output contract SHALL be: `brainstorm`
produces `brainstorm.md` as raw divergent exploration; `proposal` produces
`proposal.md` with Why, What Changes (BREAKING-marked), Capabilities (new
kebab-case names plus modified existing specs checked against
`openspec/specs/`), and Impact; `specs` writes one spec file per named
capability using ADDED/MODIFIED/REMOVED/RENAMED delta ops with exactly-four-`####`
Scenario headings; `design` produces `design.md` with Context, Goals/Non-Goals,
Decisions (each with rationale and alternatives), Risks/Trade-offs, and a
Migration Plan covering every requirement in `specs/`; `tasks` produces
`tasks.md` with `##` numbered groups of `- [ ] X.Y` checkboxes; `plan` produces
`plan.md` with concrete file paths and complete code blocks and no placeholder
patterns. Evidenced by the six phase calls and their `produce=`/`check=` text in
`.steer/workflows/openspec-propose.steer`.

#### Scenario: each phase writes its declared artifact under the change directory

- **WHEN** a phase runs
- **THEN** it persists its artifact under `openspec/changes/{change}/` at the
  path named in its `produce=` list and the next phase reads that artifact.

#### Scenario: the specs phase enforces exactly-four-hashtag scenario headings

- **WHEN** the `specs` phase runs
- **THEN** it writes one spec file per capability with delta ops and its `check=`
  requires every requirement to have at least one `#### Scenario` heading using
  exactly four hashtags.

### Requirement: The design phase covers every spec requirement

The `design` phase's `check=` SHALL require that `design.md` contains Context,
Goals/Non-Goals, Decisions (each with rationale and alternatives),
Risks/Trade-offs, and a Migration Plan, and that it covers every requirement in
`specs/`. Evidenced by the `design(...)` call's `check=` text in
`.steer/workflows/openspec-propose.steer`.

#### Scenario: design cannot leave a spec requirement uncovered

- **WHEN** the `design` phase runs
- **THEN** its `check=` requires the Migration Plan and Decisions to cover every
  requirement recorded in `specs/`.

### Requirement: The plan phase forbids placeholder patterns

The `plan` phase's `check=` SHALL require `plan.md` to contain concrete tasks
with exact file paths and complete code blocks and to contain no placeholder
patterns (TBD/TODO/"implement later"). Evidenced by the `plan(...)` call's
`check=` text in `.steer/workflows/openspec-propose.steer`.

#### Scenario: a placeholder pattern fails the plan check

- **WHEN** `plan.md` contains a TBD, TODO, or "implement later" placeholder
- **THEN** the `plan` phase's `check=` fails because concrete file paths and code
  blocks are required.

### Requirement: The workflow ends with an interactive review pause, not auto-apply

After the `plan` phase the workflow SHALL `print` an instruction directing the
user to review the artifacts in `openspec/changes/{change}/` and, when ready,
run the `openspec-apply` workflow. It SHALL NOT auto-continue into the apply
phase; the pause is an intentional human review gate. Evidenced by the final
`print(...)` call in `.steer/workflows/openspec-propose.steer`.

#### Scenario: the workflow stops for review instead of applying

- **WHEN** the `plan` phase completes
- **THEN** the workflow prints a review instruction pointing at
  `openspec/changes/{change}/` and names `openspec-apply` as the next step
  without running it.
