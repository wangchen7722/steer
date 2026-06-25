# OS Bugfix Workflow

## Purpose

The `os-bugfix` `.steer` workflow drives a generic OS-domain bugfix lifecycle for
Rust + GN/Ninja codebases at manifest (multi-repo) scale with human-in-the-loop
real-device flashing, persisting cross-iteration memory in a per-bug dossier at
`bugfix/<slug>/` and bounding the fix loop by `max_attempts`.

## Requirements

### Requirement: The workflow declares the OS-domain template, a bug slug, and a max-attempts budget

The workflow SHALL declare `@template = "os-bugfix"`, a `@context` positioning it
as an OS bugfix lifecycle for Rust/GN+Ninja multi-repo codebases with
real-device flashing whose memory persists in `bugfix/<slug>/`, and a
`@description` surfaced to `steer workflow list`. It SHALL declare the tunable
budget `max_attempts = 3` and ask exactly one per-dossier input variable —
`bug_slug` — with a kebab-case `return=` type that becomes the
`bugfix/<slug>/` dossier directory. Evidenced by the `@template`/`@context`/`@description`
directives, the `max_attempts = 3` line, and the `bug_slug = ask(...)` call in
`.steer/workflows/os-bugfix.steer`.

#### Scenario: the bug slug is the sole per-dossier input

- **WHEN** the workflow runs
- **THEN** it asks the `bug_slug` once with a kebab-case `return=` type whose
  value becomes the `bugfix/<slug>/` dossier directory.

#### Scenario: the template and context fix the OS domain

- **WHEN** the workflow module is loaded
- **THEN** `@template` is `"os-bugfix"`, `@context` declares the Rust/GN+Ninja
  multi-repo OS domain with real-device flashing and dossier-persisted memory,
  and `max_attempts` is 3.

### Requirement: Resume detection skips completed phases and never redoes them

The workflow SHALL detect an existing dossier via a
`command("test -d bugfix/{bug_slug} ...")` call returning `yes` or `no`. When the
dossier exists, it SHALL run a resume context-load `task` that reads every
existing dossier file and states what is known, what was already attempted, and
where work stopped, explicitly NOT redoing completed phases. Each subsequent
phase (reproduce, rca, capture_commands) SHALL be guarded by a
`command("test -f bugfix/{bug_slug}/<file> ...")` skip so a completed phase is
never redone. The [`runtime-check-gate`](openspec/specs/runtime-check-gate/spec.md)
engine capability backs these `command(...)` resumability checks. Evidenced by
the `exists = command(...)` resume branch and the `has_repro`/`has_rca`/`has_commands`
`test -f` guards in `.steer/workflows/os-bugfix.steer`.

#### Scenario: an existing dossier triggers a resume context load

- **WHEN** `bugfix/{bug_slug}` already exists
- **THEN** the workflow runs a resume context-load task that reads the prior
  dossier and states where work stopped without redoing completed phases.

#### Scenario: a phase with an existing output file is skipped

- **WHEN** `bugfix/{bug_slug}/reproduce.md` (or `rca.md`, or `flash.md`) already
  exists
- **THEN** the corresponding phase is skipped via its `test -f` guard so a
  completed phase is never redone.

### Requirement: Each phase persists a named dossier artifact with a per-phase output contract

The workflow SHALL persist a per-phase output under `bugfix/<slug>/` with the
contract: `intake` writes `intake.md` (slug, title, component/crate, severity,
OS/build version, target device, environment, reporter/date, status);
`reproduce` writes `reproduce.md` (concrete steps, observed-vs-expected,
environment, captured log paths) and appends a dated entry to `findings.md`;
`rca` writes `rca.md` and appends to `findings.md`; `capture_commands` writes
`flash.md` (target device + build command + flash command) and `verify.md`
(verify command), all user-sourced and captured once. `findings.md` and
`iterations.md` SHALL be append-only logs read on every attempt and every
resume. Evidenced by the phase calls and their `produce=` lists in
`.steer/workflows/os-bugfix.steer`.

#### Scenario: each phase writes its declared dossier file

- **WHEN** a phase runs
- **THEN** it persists its artifact under `bugfix/{bug_slug}/` at the path named
  in its `produce=` list.

#### Scenario: findings and iterations are append-only memory

- **WHEN** the reproduce, rca, or iteration_log phase runs
- **THEN** it appends a dated entry to `findings.md` (and `iterations.md` for the
  fix loop), and later attempts and resumes read those logs.

### Requirement: The rca phase names affected git projects and the failing code location

The `rca` phase's `check=` SHALL require `rca.md` to name the affected git
project(s), the failing crate/module/function/file/line, the causal chain, the
observed-vs-expected divergence, and the evidence, and to append a dated RCA
entry to `findings.md`. Evidenced by the `rca(...)` call and its `check=` text in
`.steer/workflows/os-bugfix.steer`.

#### Scenario: rca identifies the affected project and failing location

- **WHEN** the `rca` phase runs
- **THEN** its `check=` requires `rca.md` to name the affected git project(s) and
  the failing crate/module/function/file/line plus the causal chain.

### Requirement: The bounded fix loop designs a different fix each attempt and is bounded by max_attempts

The workflow SHALL run the fix loop as a `loop ... until` with `attempt`
starting at 0, incrementing once per iteration, and `fixed` starting at false.
Each iteration SHALL run `develop_fix` (which reads `rca.md`, `iterations.md`,
and `findings.md` and devises a fix that DIFFERS from prior failed attempts),
a build `task` (running the cached build command from `flash.md`), a `flash`
phase (human-in-the-loop, confirming device readiness pre-flash and post-flash
reboot), a `verify` phase (running the cached verify command or asking the human
to run it on-device and appending the result to `verify.md`), a
`repro_gone = judge(...)` based on the verify evidence, and an `iteration_log`
that appends the attempt to `iterations.md` and any new finding to `findings.md`.
The loop SHALL set `fixed = repro_gone` and terminate when `fixed` is true or
`attempt >= max_attempts` (3). Each iteration reads prior `iterations.md` and
`findings.md`, so it never repeats a failed fix. Evidenced by the
`attempt`/`fixed`/`loop`/`until` block in `.steer/workflows/os-bugfix.steer`.

#### Scenario: each attempt designs a fix different from prior failures

- **WHEN** `develop_fix` runs on a given attempt
- **THEN** it reads `iterations.md` and `findings.md` and devises a fix that
  differs from prior failed attempts.

#### Scenario: the loop stops as soon as the bug is fixed

- **WHEN** a verify-evidence `judge` sets `fixed = true`
- **THEN** the `until` condition is satisfied and the loop terminates before
  `attempt` reaches `max_attempts`.

#### Scenario: the loop stops at the attempt budget even if unfixed

- **WHEN** `attempt` reaches `max_attempts` (3) and `fixed` is still false
- **THEN** the loop terminates by budget and the workflow proceeds to handoff.

### Requirement: The flash step is a human-in-the-loop gate

The `flash` phase SHALL read the flash command and device profile from
`flash.md` and require the human to confirm device readiness before flashing and
post-flash reboot afterward via `AskUserQuestion`. A not-ready answer SHALL fail
the step's `check` and retry the step. Evidenced by the `flash(...)` call and its
`check=` text in `.steer/workflows/os-bugfix.steer`.

#### Scenario: a not-ready human answer fails the flash check

- **WHEN** the human answers not-ready at the pre-flash or post-flash gate
- **THEN** the flash step's `check` fails and the step retries.

### Requirement: The workflow finalizes on success or hands off on exhaustion

When the loop exits with `fixed = true`, the workflow SHALL run a finalize
`task` that reviews every changed file across the affected git project(s)
recorded in `rca.md`, confirms each change is justified by the root cause, sets
`intake.md` status to FIXED, and records the final commit or patch refs in
`fix.md`, then prints a success message. When the loop exits by budget with
`fixed = false`, it SHALL run `handoff` producing `handoff.md` (root cause, every
attempted fix and why it failed, the last failing evidence, and the next
diagnostic step), then prints a halt message. Evidenced by the `if fixed` /
`else` / `handoff(...)` block in `.steer/workflows/os-bugfix.steer`.

#### Scenario: a fixed bug is finalized across affected projects

- **WHEN** the loop exits with `fixed = true`
- **THEN** the finalize task sets `intake.md` status to FIXED, records final
  commit or patch refs in `fix.md`, and confirms every changed file maps to an
  affected project in `rca.md`.

#### Scenario: an exhausted budget writes a handoff

- **WHEN** the loop exits by budget with `fixed = false`
- **THEN** `handoff.md` is written stating the root cause, every attempted fix
  and why it failed, the last failing evidence, and the next diagnostic step.
