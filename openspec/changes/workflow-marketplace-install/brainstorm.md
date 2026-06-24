# Brainstorm — `workflow-marketplace-install`

> Raw capture of the collaborative brainstorming session for the
> `steer workflow install` marketplace feature. This is a verbatim record of the
> exploration, the questions asked, the answers given, and the design direction
> that emerged — not a restructured spec. The structured proposal, specs, and
> design live in their own files under this change directory.

## The idea (as stated by the user)

Implement a **workflow marketplace**. `steer workflow install` should:

1. Read a GitHub repository URL from an **environment variable**.
2. `steer workflow install` (default) **or** `steer workflow install --marketplace "xxxx"`.
3. **Clone the repo into a temp folder** automatically.
4. The **CLI becomes an interactive, scrollable window**: the user presses
   **space** to toggle which workflows to install, **enter** to confirm.
5. On confirm, **download the selected workflows and their associated
   templates** into the current project's `.steer/` directory.
6. **Handle conflicts**: when a target file already exists, prompt the user for
   how to resolve it.

The user also explicitly asked: **design the marketplace repository structure.**

## Context explored

- **steer's #1 design principle** (`docs/design.md`, README): "steer never
  touches the outside world." steer only runs control flow, renders
  instructions, and manages state. It does **not** run shell, read/write the
  filesystem, spawn processes, or touch the terminal. All real work is done by
  the agent; results flow back via `steer instance set`.
- **Existing `workflow` subcommands** (`crates/steer-cli/src/main.rs`):
  `validate`, `simulate`, `list`. They are **read-only host-side tools** that
  read workflow definitions. `list` scans a directory and parses each `.steer`
  for `@description`.
- **`.steer/` layout**: `workflows/<name>.steer` (flat, one file per workflow),
  `templates/<set>/<node>.j2.md` (template sets selected by `@template`).
- **The recently-added `workflow-list-command` change** established the
  OpenSpec conventions for this repo and the `@description` directive +
  `workflow_description(&Module)` helper, which the installer can **reuse** to
  read each marketplace workflow's description with zero new parsing.
- **Deps today**: very lean — `clap`, `thiserror`, `serde`, `serde_json`, and
  the three internal crates. No terminal, git, or HTTP deps.

## The central tension (the thing this brainstorm turned on)

The request — clone a repo, render an interactive TUI, download files —
**directly engages the outside world**, which is exactly what steer's core
principle forbids. This was raised with the user and is the foundational
decision.

**Resolution chosen:** treat `install` (and `uninstall`) as steer's
**package-manager side**, by analogy to `cargo install` vs `cargo run`. The
"never touch the outside world" principle governs the **runtime execution loop**
(`instance start/step/check/set`) — there, the agent does the work and steer
only renders instructions. `install` runs **outside any instance**, as a
one-time host-side setup operation. It is a **bounded, documented exception**:
only the `install`/`uninstall` setup commands touch git/the terminal/the
filesystem; the instance runtime stays pure. This keeps the principle intact
where it matters (the agent-driven loop) while giving users a real installer.

## Questions asked and answers given

**Q1 — Architecture framing.** The request conflicts with steer's #1 principle.
How to reconcile?
- **Answer: "Package-manager command" (recommended).** `install` is a
  host-side package-manager command; a bounded exception to the principle for
  setup commands only; the instance runtime loop stays pure. The CLI itself
  clones, renders the TUI, and copies files. (This matches the user's original
  description.)

**Q2 — Selection UI.** How to implement the interactive multi-select window?
- **Answer: `ratatui` + `crossterm` (recommended).** A real scrollable list with
  space-to-toggle / enter-to-confirm / arrow navigation. Cost: +2 dependencies
  (a notable, accepted footprint increase for an otherwise lean workspace).

**Q3 — Clone method.** How to obtain the repo?
- **Answer: shell out to system `git` (recommended).** `std::process::Command`
  `git clone` into a temp dir. Zero new Rust deps; auth/SSH/HTTPS delegated to
  git's own config. Requires git on the host (essentially always present for
  steer's audience).

**Q4 — How the repo address is expressed** (user mentioned both an env var and
`--marketplace "xxxx"`).
- **Answer: env var + named registry (recommended).** Default URL from env var;
  `--marketplace <name>` resolves against a local registry file; `--marketplace
  <url>` accepted directly.

**Q5 — Conflict handling.** When a target workflow/template already exists?
- **Answer: per-file interactive prompt (recommended).** Per-conflict
  `[s]kip` / `[o]verwrite` / `[b]ackup` + `[S]`/`[O]` apply-to-all, default
  skip. Global flags `--force` / `--skip` / `--dry-run` for non-interactive use.

## Approaches considered

- **A. Package-manager command (chosen).** CLI does clone + TUI + copy. Best
  UX, matches the request. Documented exception to the principle for setup
  commands only.
- **B. Instruction-based (agent executes).** CLI stays inert; renders clone /
  select / copy as instructions for the agent; selection via `AskUserQuestion`.
  Most faithful to the principle, but there is no "interactive window in the
  CLI" — rejected by the user.
- **C. Hybrid.** CLI does only mechanical host bits (clone, copy); selection and
  conflict resolution rendered as instructions to the agent. Middle ground;
  rejected in favor of A for UX.

## Proposed marketplace repository structure

**Key insight:** the marketplace repo should **mirror the local `.steer/`
layout** so it is just a standard steer project tree, and the installer can
**reuse the existing `workflow_description` parser** to read each workflow's
`@description` with no manifest required.

```
steer-marketplace/                     # any git repo
├── README.md                          # human-facing catalog (optional)
├── workflows/                         # mirrors .steer/workflows/
│   ├── openspec-propose.steer         # each has @description + @template
│   ├── openspec-apply.steer
│   └── os-bugfix.steer
└── templates/                         # mirrors .steer/templates/
    ├── openspec-superpowers/
    │   ├── brainstorm.j2.md
    │   ├── proposal.j2.md
    │   └── ...
    └── os-bugfix/
        ├── intake.j2.md
        └── ...
```

**Installation rule (no manifest needed):** to install a workflow, copy its
`.steer` file into `.steer/workflows/`, and copy **every template set named in
its `@template` directive** into `.steer/templates/<set>/`. The `@template`
directive is the authoritative link between a workflow and its templates, so the
installer resolves it deterministically — no separate manifest to keep in sync.
A workflow with no `@template` installs just its `.steer` file.

This means **any existing steer project can serve as a marketplace** by being
pushed to a git repo; no special authoring is required.

## Proposed install flow

1. **Resolve the marketplace URL**, in priority order:
   1. `--marketplace <url>` — if the value looks like a URL (contains `://` or
      ends in `.git`), use it directly.
   2. `--marketplace <name>` — look the name up in `.steer/marketplaces.toml`
      (project-local), then `~/.steer/marketplaces.toml` (user-global); first
      match wins.
   3. no `--marketplace` — read env var `STEER_MARKETPLACE_URL`.
   4. none of the above — error with guidance (set the env var or register a
      named marketplace).
2. **Clone** with `git clone --depth 1 [--ref <r>] <url> <tmp>` into a temp dir
   (cleaned up on exit). `--ref` lets the user pin a branch/tag/commit.
3. **Scan** `<tmp>/workflows/*.steer`; parse each for `@description` and
   `@template` (reusing `steer_core::workflow_description` + a new
   `workflow_template(&Module) -> Option<String>` helper).
4. **Render the selection TUI** (ratatui + crossterm): a scrollable list, one
   row per workflow — `[ ] name  — description`. Keys: **space** toggle,
   **↑/↓** (and `j`/`k`) scroll, `a` select-all, `n` select-none, **enter**
   confirm, `q`/`Esc` cancel. A footer shows the keybindings. Each row can note
   the template set(s) it will bring.
5. **Pre-flight conflict scan** against the current `.steer/`. For each conflict,
   prompt inline (after the TUI closes): `[s]kip / [o]verwrite / [b]ackup`
   (+`[S]`/`[O]` apply-to-all), default skip.
6. **Copy** selected `.steer` files → `.steer/workflows/`, and each referenced
   template-set dir → `.steer/templates/<set>/`. Create `.steer/` and
   subdirs if missing. Backup renames the existing file to `.bak` before
   overwriting.
7. **Print a summary** (installed / skipped / backed up), then clean up the temp
   dir. Exit 0 on success, non-zero on cancel or fatal error.

**Non-interactive / agent / CI flags** (so the command is scriptable, not just
human-interactive): `--workflows a,b,c` (install specific, skip the TUI),
`--all` (install every workflow, no TUI), `--force` / `--skip` / `--backup`
(conflict policy, no prompts), `--dry-run`, `--ref <r>`. When stdout is not a
TTY and no selection flag is given, error with guidance rather than hang.

## Named registry format (`.steer/marketplaces.toml`)

A small TOML file mapping names to URLs (project-local `.steer/marketplaces.toml`
overrides user-global `~/.steer/marketplaces.toml`):

```toml
[marketplaces.official]
url = "https://github.com/wangchen7722/steer-marketplace"

[marketplaces.community]
url = "https://github.com/foo/steer-community"
```

TOML (not JSON) for human-edited config; adds the lightweight pure-Rust `toml`
crate. Home dir resolved via `HOME`/`USERPROFILE` (no `dirs` crate).

## New dependencies (accepted trade-off)

- `ratatui` + `crossterm` — the interactive selection TUI.
- `toml` — parsing the named-marketplace registry.
- `git` (system binary, not a Rust crate) — cloning.
- No HTTP, no libgit2, no `dirs`. serde/serde_json/clap/thiserror already present.

## Scope and non-goals (v1)

**In scope:** `steer workflow install` with default/env/named/direct-URL source,
git clone to temp, ratatui multi-select, copy workflow + its `@template` sets,
per-file conflict resolution, non-interactive flags, dry-run. Documented as a
package-manager command outside the runtime loop.

**Non-goals (deferred):** `uninstall` (separate change), signature/trust
verification of marketplace contents, private-repo token auth flows (rely on
git's own credential helpers for now), installing standalone template packs not
referenced by any workflow, dependency/version resolution between workflows,
full-TUI conflict resolution screens (v1 uses inline prompts after selection),
update/upgrade of already-installed workflows (re-run install with
`--force`/`--backup`).

## Open question to confirm during design phase

- Exact env-var and flag names, default registry path, and whether the
  marketplace root may optionally live under `<repo>/.steer/` (vs. `<repo>/`
  directly) — to be settled in `design.md`.
