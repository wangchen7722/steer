# Spec — `install-conflict-resolution`

> Capability introduced by change `workflow-marketplace-install`.
> Covers detecting target files that already exist under `.steer/` and resolving
> each conflict: the interactive skip/overwrite/backup prompt (with apply-to-all
> and a default of skip), the `--force`/`--skip`/`--backup` global flags for
> non-interactive use, the non-TTY fallback, and backup file naming.

## ADDED Requirements

### Requirement: Conflicts are detected before any file is written

Before copying, the installer MUST compute the full set of target paths for the
selected workflows (each `.steer/workflows/<name>.steer` and every file under
each referenced `.steer/templates/<set>/`) and flag as a conflict any path whose
target file already exists. A template set directory that already exists MUST be
treated as conflicting for each file that would be written into it.

#### Scenario: an existing workflow file is a conflict

- **WHEN** `.steer/workflows/openspec-propose.steer` already exists and the user
  selects `openspec-propose` to install
- **THEN** that target is flagged as a conflict.

#### Scenario: an existing template set directory is a conflict

- **WHEN** `.steer/templates/openspec-superpowers/` already exists and the user
  selects a workflow that references it
- **THEN** each file that would be written into that directory is flagged as a
  conflict.

---

### Requirement: Each conflict is resolved by an interactive prompt, defaulting to skip

When running interactively (TTY) without a global conflict flag, the installer
MUST prompt once per conflict offering **s**kip (keep the existing file),
**o**verwrite (replace with the marketplace copy), and **b**ackup (rename the
existing file, then write the marketplace copy), plus **S** to skip all
remaining conflicts and **O** to overwrite all remaining. The default action on a
bare **enter** MUST be skip, so a no-op confirm never destroys an existing file.

#### Scenario: skip keeps the existing file

- **WHEN** a conflict prompt is answered with `s`
- **THEN** the existing file is kept and the marketplace copy is not written.

#### Scenario: overwrite replaces the existing file

- **WHEN** a conflict prompt is answered with `o`
- **THEN** the existing file is replaced by the marketplace copy.

#### Scenario: backup preserves then replaces

- **WHEN** a conflict prompt is answered with `b`
- **THEN** the existing file is renamed to a backup name and the marketplace copy
  is written in its place.

#### Scenario: apply-to-all resolves the remaining conflicts at once

- **WHEN** a conflict prompt is answered with `S` (or `O`)
- **THEN** that conflict and every subsequent conflict is resolved as skip (or
  overwrite) with no further prompts.

#### Scenario: a bare enter skips

- **WHEN** a conflict prompt is answered with enter alone
- **THEN** the conflict is resolved as skip.

---

### Requirement: Global flags make conflict resolution non-interactive

`--force` MUST resolve every conflict as overwrite with no prompts; `--skip`
MUST resolve every conflict as skip with no prompts; `--backup` MUST resolve
every conflict as backup with no prompts. These flags apply to all conflicts and
remove the need for a TTY during conflict resolution.

#### Scenario: `--force` overwrites all conflicts silently

- **WHEN** the user runs `steer workflow install --marketplace <url> --all --force`
  against targets that already exist
- **THEN** every conflicting file is overwritten and no conflict prompt is shown.

#### Scenario: `--skip` leaves existing files untouched

- **WHEN** the user runs with `--skip` against existing targets
- **THEN** every conflict is skipped and the existing files are unchanged.

#### Scenario: `--backup` backs up all conflicts

- **WHEN** the user runs with `--backup` against existing targets
- **THEN** every conflicting file is renamed to a backup before writing, with no
  prompt.

---

### Requirement: Without a TTY and without a global flag, conflicts default to skip

When stdout is not a TTY and none of `--force`/`--skip`/`--backup` is passed, the
installer MUST resolve every conflict as skip and MUST print which targets were
skipped, so a non-interactive run never destroys an existing file and never
blocks on a prompt.

#### Scenario: a non-interactive run skips conflicts and reports them

- **WHEN** a non-TTY run encounters conflicts and no global conflict flag is set
- **THEN** each conflict is skipped, the skipped targets are reported, and the
  command does not block.

---

### Requirement: Backups use a `.bak` suffix with numeric disambiguation

When resolving a conflict as backup, the installer MUST rename the existing file
to `<path>.bak`. If `<path>.bak` already exists, it MUST use successive numeric
suffixes (`<path>.bak`, then `<path>.bak.1`, `<path>.bak.2`, ...) so no existing
backup is ever clobbered.

#### Scenario: the first backup takes the `.bak` name

- **WHEN** a conflict is backed up and no `<path>.bak` exists
- **THEN** the existing file is renamed to `<path>.bak`.

#### Scenario: a later backup does not clobber an earlier one

- **WHEN** a conflict is backed up and `<path>.bak` already exists
- **THEN** the existing file is renamed to `<path>.bak.1` (and so on), leaving
  the earlier `<path>.bak` intact.
