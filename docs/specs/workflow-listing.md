# Workflow Listing

> Behavior specs for `steer workflow list`: enumerating the workflow catalog and
> the `@description` directive that annotates catalog entries.

## Scenario: `list` enumerates `.steer/workflows/` by default
- **WHEN** the user runs `steer workflow list` with no argument
- **THEN** the CLI scans `.steer/workflows/`, prints one line per `*.steer` file
  (name + description), and exits successfully.

## Scenario: a custom directory argument is honored
- **WHEN** the user runs `steer workflow list <dir>`
- **THEN** the CLI enumerates `*.steer` files under `<dir>` instead of
  `.steer/workflows/`.

## Scenario: entries are sorted by name, non-`.steer` files skipped
- **WHEN** the directory contains `zebra.steer`, `alpha.steer`, and `notes.md`
- **THEN** `list` prints `alpha` before `zebra` and omits `notes.md`.

## Scenario: the listed name is the file stem
- **WHEN** the directory contains `openspec-propose.steer`
- **THEN** the entry's name is `openspec-propose` (no `.steer` extension).

## Scenario: a present `@description` is shown
- **WHEN** `alpha.steer` contains `@description = "Alpha workflow"`
- **THEN** the `alpha` entry prints `Alpha workflow` as its description.

## Scenario: an absent or empty description prints a placeholder
- **WHEN** a workflow has no `@description`, or has `@description = ""`
- **THEN** its entry prints `(no description)`.

## Scenario: an unparseable workflow is still listed
- **WHEN** a `.steer` file fails to parse
- **THEN** `list` prints its name with an `(unparseable)` marker and does not
  abort the listing.

## Scenario: a missing or empty directory is not an error
- **WHEN** the target directory does not exist or has no `*.steer` files
- **THEN** `list` prints `(no workflows in <dir>)` and exits successfully.

## Scenario: `@description` is runtime-inert
- **WHEN** a workflow containing `@description = "..."` is validated, simulated,
  or run as an instance
- **THEN** it behaves identically to the same workflow with the directive
  removed; the description is shown only by `list`, never by `start`/`status`.
