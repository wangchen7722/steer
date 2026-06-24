# Spec — `interactive-install-selection`

> Capability introduced by change `workflow-marketplace-install`.
> Covers the ratatui + crossterm scrollable multi-select UI that
> `steer workflow install` uses to let a user choose which workflows to install,
> the keybindings it honors, when it is shown versus skipped, and the non-TTY
> fallback.

## ADDED Requirements

### Requirement: The selection UI is shown only on a TTY without selection flags

When the process's stdout is a TTY and neither `--workflows` nor `--all` is
passed, the installer MUST render an interactive, scrollable, multi-select list
showing one row per catalog workflow. When stdout is not a TTY (piped, CI,
agent-driven) and no selection flag is passed, the installer MUST NOT attempt to
open the UI; it MUST print guidance to use `--workflows` or `--all` and exit
non-zero, rather than block waiting for input.

#### Scenario: an interactive terminal gets the UI

- **WHEN** stdout is a TTY and the user runs `steer workflow install --marketplace <url>` with no selection flag
- **THEN** the interactive multi-select list is rendered.

#### Scenario: a non-TTY environment does not hang

- **WHEN** stdout is not a TTY and no `--workflows`/`--all` flag is passed
- **THEN** the installer prints guidance naming `--workflows`/`--all`, performs
  no install, and exits non-zero.

#### Scenario: a selection flag skips the UI even on a TTY

- **WHEN** stdout is a TTY and the user passes `--all`
- **THEN** no interactive UI is rendered; the install proceeds directly.

---

### Requirement: Each row shows selection state, name, description, and template sets

Each catalog row MUST display a checkbox reflecting the workflow's selected
state, the workflow's name (file stem), its catalog description (or a
placeholder when absent), and the template set(s) that selecting it would
install (read from its `@template` directive), so the user can see the install
blast radius before confirming.

#### Scenario: a workflow's template sets are visible in its row

- **WHEN** the catalog contains `openspec-propose` with
  `@template = "openspec-superpowers"`
- **THEN** that row indicates selecting it will install the
  `openspec-superpowers` template set.

#### Scenario: a workflow with no description shows a placeholder

- **WHEN** a catalog workflow has no `@description`
- **THEN** its row shows a placeholder description, not a blank.

---

### Requirement: The UI honors a fixed set of keybindings

The selection UI MUST support these keys: **space** toggles the highlighted
workflow's selection; **enter** confirms the current selection and proceeds to
install; **up**/**down** (and `j`/`k`) move the highlight and scroll the list
when it exceeds the viewport; `a` selects all workflows; `n` clears the
selection; **q** or **Esc** cancels, installing nothing.

#### Scenario: space toggles a row

- **WHEN** the highlight is on a deselected row and the user presses space
- **THEN** that row becomes selected; pressing space again deselects it.

#### Scenario: enter confirms and proceeds

- **WHEN** the user presses enter with one or more workflows selected
- **THEN** the UI exits and the installer proceeds to install the selected
  workflows.

#### Scenario: scrolling moves the highlight past the viewport

- **WHEN** the catalog has more rows than the viewport shows and the user presses
  down past the last visible row
- **THEN** the list scrolls so the highlight remains visible.

#### Scenario: `a` and `n` select and clear all

- **WHEN** the user presses `a`
- **THEN** every workflow becomes selected; pressing `n` then clears every
  selection.

#### Scenario: q or Esc cancels

- **WHEN** the user presses q or Esc
- **THEN** the UI exits, nothing is installed, and the command exits 0 with a
  "cancelled" message.

---

### Requirement: Confirming with no selection installs nothing

Pressing **enter** with zero workflows selected MUST NOT be an error: the
installer MUST exit 0, install nothing, and print a message that nothing was
selected.

#### Scenario: confirming an empty selection is a clean no-op

- **WHEN** the user presses enter with no rows selected
- **THEN** no files are written and the command exits 0 with a "nothing
  selected" message.
