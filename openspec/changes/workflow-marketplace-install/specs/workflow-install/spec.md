# Spec — `workflow-install`

> Capability introduced by change `workflow-marketplace-install`.
> Covers the `steer workflow install` command end to end: cloning the marketplace
> to a temp dir, scanning the catalog, copying each selected workflow and its
> `@template` template sets into `.steer/`, the non-interactive selection flags,
> `--ref`/`--dry-run`, directory creation, the install summary, temp cleanup, and
> exit status. Selection UX and conflict handling are separate capabilities.

## ADDED Requirements

### Requirement: The marketplace is shallow-cloned into a temporary directory

`steer workflow install` MUST clone the resolved marketplace URL into a fresh
temporary directory (a shallow clone, `--depth 1`) and treat that clone as the
read-only source for the rest of the operation. The temporary directory MUST be
removed when the command exits, whether on success, cancellation, or error. If
`--ref <r>` is passed, the clone MUST target that branch, tag, or commit.

#### Scenario: a successful clone is scanned

- **WHEN** the resolved URL is reachable
- **THEN** the installer clones it into a temp dir and scans its catalog from
  there.

#### Scenario: `--ref` pins the cloned ref

- **WHEN** the user runs `steer workflow install --marketplace <url> --ref v2`
- **THEN** the clone checks out ref `v2`.

#### Scenario: the temp directory is removed on success

- **WHEN** an install completes successfully
- **THEN** the temp clone directory no longer exists.

#### Scenario: the temp directory is removed on failure

- **WHEN** an install fails after cloning (for example, a write error)
- **THEN** the temp clone directory is still removed.

---

### Requirement: A missing or failing clone is a fatal error

If the `git` binary is not on `PATH`, or the clone fails (bad URL, unreachable
host, authentication required and not available), the installer MUST print an
error message and exit with a non-zero status without writing anything to
`.steer/`. The temp directory MUST still be cleaned up.

#### Scenario: git is not installed

- **WHEN** the `git` binary cannot be found on `PATH`
- **THEN** the installer prints an error stating git is required and exits
  non-zero, with no clone attempted.

#### Scenario: an unreachable URL fails the clone

- **WHEN** the resolved URL cannot be cloned
- **THEN** the installer prints the clone failure and exits non-zero, leaving
  `.steer/` untouched and removing the temp directory.

---

### Requirement: The catalog is scanned from the cloned tree

The installer MUST locate the marketplace catalog base as the first of:
`<root>/.steer` if it contains a `workflows/` or `templates/` directory, else
`<root>`. It MUST then scan `<base>/workflows/*.steer` (flat, top-level files
only). Each workflow's name is its file stem; its catalog description is its
`@description` (absent → a placeholder); its template sets are read from its
`@template` directive. A marketplace whose catalog base has no `.steer` files is
not an error: the installer prints a "no workflows found" notice and exits 0.

#### Scenario: shipped workflows populate the catalog

- **WHEN** the clone contains `workflows/openspec-propose.steer`,
  `workflows/openspec-apply.steer`, and `workflows/os-bugfix.steer`
- **THEN** the catalog lists all three by their file stems.

#### Scenario: a `.steer`-rooted repo is supported

- **WHEN** the clone has no top-level `workflows/` but has
  `.steer/workflows/*.steer`
- **THEN** the installer uses `.steer` as the catalog base and lists those
  workflows.

#### Scenario: a workflow without `@description` is still listed

- **WHEN** `workflows/alpha.steer` has no `@description` directive
- **THEN** `alpha` appears in the catalog with a placeholder description.

#### Scenario: an empty marketplace is a no-op success

- **WHEN** the cloned catalog base contains no `.steer` files
- **THEN** the installer prints a "no workflows found" notice, installs nothing,
  and exits 0.

---

### Requirement: Installing a workflow brings its `@template` template sets

For each selected workflow, the installer MUST copy its `.steer` file to
`.steer/workflows/<name>.steer` and MUST copy every template set named in the
workflow's `@template` directive from `<base>/templates/<set>/` to
`.steer/templates/<set>/` (the whole directory). A workflow with no `@template`
directive installs only its `.steer` file. If a referenced template set is absent
from the marketplace, the installer MUST still install the workflow's `.steer`
file and MUST print a warning naming the missing template set.

#### Scenario: a workflow with one template set installs both

- **WHEN** the selected workflow `openspec-propose.steer` declares
  `@template = "openspec-superpowers"` and the marketplace contains
  `templates/openspec-superpowers/`
- **THEN** `openspec-propose.steer` is written to `.steer/workflows/` and the
  `openspec-superpowers` directory is written under `.steer/templates/`.

#### Scenario: a workflow without `@template` installs only its file

- **WHEN** the selected workflow has no `@template` directive
- **THEN** only its `.steer` file is copied; no template directory is touched.

#### Scenario: a missing referenced template set warns but still installs

- **WHEN** a selected workflow declares `@template = "ghost"` but the marketplace
  has no `templates/ghost/`
- **THEN** the workflow's `.steer` file is installed and a warning naming `ghost`
  is printed.

---

### Requirement: `.steer/` structure is created on demand

If `.steer/`, `.steer/workflows/`, or `.steer/templates/` do not exist, the
installer MUST create them as needed before writing, so a fresh project can
receive an install without manual setup.

#### Scenario: a fresh project gains the full structure

- **WHEN** the current project has no `.steer/` directory and the user installs a
  workflow with a template set
- **THEN** `.steer/workflows/` and `.steer/templates/<set>/` are created and
  populated.

---

### Requirement: Non-interactive flags select without a TUI

`--workflows <a,b,c>` MUST install exactly the named workflows (by file stem),
skipping the interactive selection UI; an unknown name in the list MUST be a
fatal error that installs nothing. `--all` MUST install every workflow in the
catalog, also skipping the UI. These flags make the command scriptable for
agents and CI.

#### Scenario: `--workflows` installs a specific subset

- **WHEN** the user runs `steer workflow install --marketplace <url> --workflows openspec-propose,os-bugfix`
- **THEN** exactly those two workflows (and their template sets) are installed,
  with no interactive UI shown.

#### Scenario: `--all` installs everything

- **WHEN** the user runs `steer workflow install --marketplace <url> --all`
- **THEN** every workflow in the catalog is installed, with no interactive UI.

#### Scenario: an unknown workflow name is a fatal error

- **WHEN** `--workflows nosuch` is passed and `nosuch` is not in the catalog
- **THEN** the installer prints an error naming `nosuch`, installs nothing, and
  exits non-zero.

---

### Requirement: `--dry-run` plans without writing

With `--dry-run`, the installer MUST perform source resolution, cloning, catalog
scanning, and selection (interactive or via flags) exactly as a real run, but
MUST NOT write, overwrite, back up, or delete any file under `.steer/`. Instead
it MUST print the actions it would take (which `.steer` files and template sets
would be installed, skipped, or backed up) and exit 0.

#### Scenario: dry-run reports the plan and writes nothing

- **WHEN** the user runs
  `steer workflow install --marketplace <url> --workflows openspec-propose --dry-run`
- **THEN** the installer prints the planned installs and leaves every file under
  `.steer/` unchanged.

---

### Requirement: An install prints a summary and uses clear exit statuses

On completion the installer MUST print a summary enumerating what was installed,
skipped, and backed up. Exit status MUST be: 0 for a successful install of one or
more workflows; 0 for a user cancellation at the selection UI (nothing installed,
a "cancelled" message); 0 for an empty catalog; non-zero for any fatal error
(no source, missing git, clone failure, unknown `--workflows` name, malformed
registry).

#### Scenario: a successful install summarizes and exits 0

- **WHEN** one or more workflows install successfully
- **THEN** the summary lists each installed workflow and template set and the
  process exits 0.

#### Scenario: cancelling the selection UI exits 0

- **WHEN** the user cancels the interactive selection
- **THEN** nothing is installed, a "cancelled" message is printed, and the
  process exits 0.
