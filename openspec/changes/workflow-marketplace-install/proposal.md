# Proposal — `workflow-marketplace-install`

## Why

steer ships and discovers workflows as `.steer` files under `.steer/workflows/`,
but **there is no way to obtain workflows from outside the project**. Today a
user who wants a workflow someone else published must manually find the repo,
clone it, and copy `.steer` files plus their template sets into the right places
under `.steer/` — error-prone busywork (templates live in a separate
`.steer/templates/<set>/` tree, and the link between a workflow and its templates
is the `@template` directive, easy to miss by hand). There is no notion of a
**workflow marketplace**: a discoverable catalog the user can browse and install
from interactively.

The opportunity is to make steer's ecosystem shareable the way package managers
do: a user runs one command, sees an interactive catalog, picks what they want,
and the tool fetches the workflow **and its templates** into `.steer/`. This
matters now because steer is agent-driven and the value of the tool is in its
workflow library — friction-free distribution of workflows directly grows that
library's reach.

This is the **first time steer would touch the outside world** (git, the
terminal, the filesystem-write path). That is a deliberate, documented
exception: `install` is a **package-manager command**, by analogy to
`cargo install` vs `cargo run`. The "steer never touches the outside world"
principle governs the **runtime execution loop** (`instance start/step/check`);
`install` runs **outside any instance** as one-time host-side setup. The
runtime loop stays pure. See `design.md` for the precise boundary.

## What Changes

- **New CLI subcommand** `steer workflow install` that fetches a marketplace
  repo, presents an **interactive multi-select catalog** (ratatui + crossterm),
  and copies the selected workflows **and the template sets each references via
  `@template`** into the current project's `.steer/`.
- **Marketplace source resolution**: a default URL from the
  `STEER_MARKETPLACE_URL` env var; a named registry
  (`.steer/marketplaces.toml`, project-local overriding user-global) queried by
  `--marketplace <name>`; and `--marketplace <url>` for a direct URL.
- **Marketplace repo structure** (defined by this change): a git repo whose root
  mirrors `.steer/` — `workflows/*.steer` (each annotated with `@description`)
  and `templates/<set>/*.j2.md`. No manifest is required; the installer scans
  the tree and reads `@description` (reusing the existing parser), resolving
  templates from each workflow's `@template` directive. Any steer project pushed
  to git is therefore already a valid marketplace.
- **Conflict resolution**: when a target `.steer`/template file already exists,
  a **per-file interactive prompt** offers skip / overwrite / backup (+apply-to-
  all), defaulting to skip.
- **Non-interactive flags** so the command is scriptable by agents and CI:
  `--workflows a,b,c`, `--all`, `--force`, `--skip`, `--backup`, `--dry-run`,
  `--ref <branch|tag|commit>`.
- **Docs**: README CLI section, `docs/specs/` behavior specs, and a documented
  trust note (installing a workflow means the agent will later execute its
  instructions; v1 relies on explicit user action + a user-chosen source, with
  no signature verification — a stated non-goal).

Not in scope (non-goals): `uninstall`, signature/trust verification, private-
repo token auth flows (rely on git's own credential helpers), standalone
template packs not referenced by a workflow, and cross-workflow dependency/
version resolution.

## Capabilities

### New Capabilities

- `marketplace-resolution`: resolving the marketplace repository URL from
  `--marketplace <url|name>`, the named registry (`.steer/marketplaces.toml`),
  and the `STEER_MARKETPLACE_URL` env var, including the precedence order and
  the error when no source is configured.

- `workflow-install`: the `steer workflow install` command end to end — clone
  the marketplace to a temp dir, scan `workflows/*.steer`, copy each selected
  workflow and its `@template` template sets into `.steer/`, support the
  non-interactive flags (`--workflows`, `--all`, `--ref`, `--dry-run`), create
  `.steer/` subdirs as needed, print an install summary, and clean up the temp
  dir. This is the orchestrator; selection and conflict handling are separate
  capabilities it composes.

- `interactive-install-selection`: the ratatui + crossterm scrollable
  multi-select TUI for choosing which workflows to install (space toggle, enter
  confirm, scroll, select-all/none, cancel), and the TTY detection that decides
  when to show the TUI vs. require a non-interactive flag.

- `install-conflict-resolution`: detecting pre-existing target files and
  resolving each via the skip/overwrite/backup prompt (+apply-to-all, default
  skip) and the `--force`/`--skip`/`--backup` global flags.

### Modified Capabilities

_(None. `openspec/specs/` currently defines no capabilities. The existing
`workflow` subcommands (`validate`/`simulate`/`list`), path resolution,
validation, simulation, and the instance runtime are unchanged. A new internal
helper `workflow_template(&Module) -> Option<String>` is added to read
`@template`, but that is an implementation detail, not a spec-level behavior
change.)_

## Impact

- **Code**
  - New install machinery, most likely a dedicated crate
    `crates/steer-marketplace` (keeps the heavy terminal/git-touching code and
    its dependencies out of the pure `steer-core`/`steer-syntax`, reinforcing
    the runtime-purity boundary) — final placement decided in `design.md`.
    Modules: source resolution, git clone, catalog scan, TUI selection, conflict
    resolution, copy orchestrator.
  - `crates/steer-cli/src/main.rs` — new `WorkflowAction::Install { ... }`
    variant and a thin `run_install` handler that delegates to the new crate.
  - `crates/steer-core/src/storage.rs` — new pure
    `pub fn workflow_template(&Module) -> Option<String>`, co-located with
    `workflow_description` (same `eval_literal().render()` pattern).
  - `crates/steer-core/src/lib.rs` — re-export `workflow_template`.
- **APIs** — one new CLI subcommand (`steer workflow install`) and its flags;
  no library API breakage. The `workflow_template` helper is a new public fn in
  `steer-core`.
- **Dependencies** — adds `ratatui`, `crossterm`, and `toml` (all pure-Rust,
  widely used). Relies on the system `git` binary for cloning. **No** HTTP,
  libgit2, or `dirs` crate; serde/serde_json/clap/thiserror already present.
- **Docs** — `README.md` (CLI table + new command + marketplace repo layout +
  `STEER_MARKETPLACE_URL` / `marketplaces.toml`); new `docs/specs/` behavior
  specs per the project's index convention; a trust/security note.
- **Performance** — negligible: one shallow `git clone` and a handful of small
  file copies.
- **Security** — installing a workflow means an agent will later execute its
  instructions, so a malicious marketplace could be harmful. v1 mitigation:
  install is an explicit user action against a user-chosen source, and the
  catalog shows each workflow's `@description` before selection. Signature/
  trust verification and sandboxing are explicit non-goals for v1, documented as
  such.
