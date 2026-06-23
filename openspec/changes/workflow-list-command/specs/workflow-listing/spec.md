# Spec — `workflow-listing`

> Capability introduced by change `workflow-list-command`.
> Covers the `steer workflow list` command and the `@description` directive.

## ADDED Requirements

### Requirement: List enumerates workflows in a directory

`steer workflow list` enumerates `*.steer` files in a target directory and prints
one entry per workflow. With no argument the target directory MUST be
`.steer/workflows/`. An optional positional `<dir>` argument MUST override the
default. Enumeration MUST be flat (top-level files only; subdirectories are
ignored) and entries MUST be sorted alphabetically by name. A missing or empty
directory is not an error.

#### Scenario: default directory lists all shipped workflows

- **WHEN** the user runs `steer workflow list` in a project whose
  `.steer/workflows/` contains `openspec-propose.steer`, `openspec-apply.steer`,
  and `os-bugfix.steer`
- **THEN** the command prints one line per workflow and exits successfully.

#### Scenario: a custom directory argument is honored

- **WHEN** the user runs `steer workflow list /tmp/my-workflows`
- **THEN** the command enumerates `*.steer` files under `/tmp/my-workflows`
  instead of `.steer/workflows/`.

#### Scenario: missing or empty directory reports no workflows

- **WHEN** the user runs `steer workflow list` and the target directory does not
  exist or contains no `*.steer` files
- **THEN** the command prints a `no workflows in <dir>` notice and exits
  successfully (exit code 0).

#### Scenario: entries are sorted alphabetically

- **WHEN** `.steer/workflows/` contains `zebra.steer` and `alpha.steer`
- **THEN** `alpha` is listed before `zebra` in the output.

#### Scenario: non-.steer files are ignored

- **WHEN** the target directory contains `notes.md` alongside `real.steer`
- **THEN** only `real` appears in the listing.

---

### Requirement: The listed name is the workflow file stem

For a workflow file `<name>.steer`, the name printed by `list` MUST be `<name>`
(the file stem, without the `.steer` extension), matching the token accepted by
`instance start`/`workflow validate`/`workflow simulate`.

#### Scenario: file stem is shown without extension

- **WHEN** the directory contains `openspec-propose.steer`
- **THEN** the entry's name is `openspec-propose`, not `openspec-propose.steer`.

---

### Requirement: Each entry shows its `@description`

For each listed workflow, `list` MUST print the value of the top-level
`@description = "..."` directive alongside the name. The description MUST be
obtained by evaluating the directive's literal and rendering it to text (empty
rendered text is treated as absent). When no `@description` is present, `list`
MUST print a `(no description)` placeholder so the description column is uniform.

#### Scenario: a present description is shown next to the name

- **WHEN** `alpha.steer` contains `@description = "Alpha workflow"`
- **THEN** the `alpha` entry prints `Alpha workflow` as its description.

#### Scenario: an absent description prints a placeholder

- **WHEN** `beta.steer` has no `@description` directive
- **THEN** the `beta` entry prints `(no description)`.

#### Scenario: an empty description is treated as absent

- **WHEN** `gamma.steer` contains `@description = ""`
- **THEN** the `gamma` entry prints `(no description)` (empty rendered text is
  absent).

#### Scenario: an unparseable workflow is still listed with a marker

- **WHEN** `broken.steer` contains a syntax error
- **THEN** `list` prints the `broken` entry with an `(unparseable)` description
  marker and does not abort the listing.

---

### Requirement: `@description` is an optional, runtime-inert directive

`@description` is a top-level `@`-directive parsed like `@template`/`@context`.
It MUST be optional. Because its only consumer is `list`, it MUST have no effect
on validation, simulation, instance start, stepping, or status — a workflow with
`@description` runs identically to one without.

#### Scenario: a workflow with `@description` validates and runs normally

- **WHEN** a workflow containing `@description = "demo"` is passed to
  `workflow validate`, `workflow simulate`, and `instance start`/`step`
- **THEN** each behaves exactly as it would with the `@description` line removed
  (validate reports OK, simulate renders its instructions, the instance runs).

#### Scenario: description is not leaked into run output

- **WHEN** an instance of a workflow with `@description = "demo"` is started
- **THEN** the `start` output does not include the description text (description
  is shown only by `list`, not by `start`/`status`).
