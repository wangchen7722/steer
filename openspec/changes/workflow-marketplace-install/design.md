# Design — `workflow-marketplace-install`

> How to implement the `steer workflow install` marketplace feature. This design
> satisfies every requirement in
> [`specs/`](./specs/) — see the Requirement coverage table at the end. It
> reorganizes the decisions captured in [`brainstorm.md`](./brainstorm.md) into
> structured sections.

## Context

steer ships and discovers workflows as `.steer` files under `.steer/workflows/`,
with their templates under `.steer/templates/<set>/`, linked by a workflow's
`@template` directive. The existing `workflow` subcommands
(`validate` / `simulate` / `list`) are **read-only host-side tools**; `list`
already parses each `.steer` for `@description` via the public
`steer_core::workflow_description(&Module)` helper. There is no way to obtain
workflows from outside the project.

steer's #1 design principle (`docs/design.md`) is "steer never touches the
outside world": it runs only control flow, renders instructions, and manages
state; it does **not** run shell, read/write the filesystem, spawn processes, or
touch the terminal. That principle governs the **runtime execution loop**
(`instance start/step/check/set`), where an external agent does the work.

**Stakeholders:** developers using steer who want to install community/published
workflows; agents and CI that need a scriptable install; the steer maintainers,
who care about preserving the runtime-purity boundary.

**Constraint central to this design:** `workflow install` must clone a git repo,
render an interactive terminal UI, and write files — all of which touch the
outside world. This is the first steer feature to do so. The design's job is to
deliver the feature while keeping the erosion of the principle **bounded,
isolated, and documented**.

## Goals / Non-Goals

**Goals:**
- `steer workflow install` that fetches a marketplace repo, presents an
  interactive multi-select catalog, and copies selected workflows **and their
  `@template` template sets** into `.steer/`.
- A defined **marketplace repo structure** (mirrors `.steer/`, no manifest) so
  any steer project pushed to git is a valid marketplace.
- Flexible **source resolution**: env var, named registry, or direct URL.
- **Conflict resolution** that never silently destroys an existing file.
- A **non-interactive contract** (`--workflows`, `--all`, `--force`, `--skip`,
  `--backup`, `--dry-run`, `--ref`) so the command is scriptable.
- Keep the runtime-purity boundary intact: confine all world-touching code to a
  new `steer-marketplace` crate; `steer-core`/`steer-syntax` stay pure.

**Non-Goals:**
- `uninstall` (separate change).
- Signature/trust verification of marketplace contents (v1 relies on explicit
  user action + a user-chosen source).
- Private-repo token-auth flows (rely on git's own credential helpers).
- Standalone template packs not referenced by any workflow.
- Cross-workflow dependency/version resolution or `update`/`upgrade`.

## Architecture

A **new crate `crates/steer-marketplace`** holds all install machinery. It
depends on `steer-syntax` + `steer-core` (read-only use: parsing `.steer` files
and the `workflow_description`/`workflow_template` helpers) plus the new
terminal/config deps (`ratatui`, `crossterm`, `toml`). `steer-cli` adds a thin
`WorkflowAction::Install` variant whose `run_install` parses CLI args and
delegates to `steer_marketplace::install(...)`. `steer-core` gains exactly one
new public helper, `workflow_template(&Module) -> Option<String>`, mirroring the
existing `workflow_description`.

Module layout inside `steer-marketplace`:

```
crates/steer-marketplace/src/
├── lib.rs        // public entry: install(InstallArgs) -> ExitCode; re-exports
├── source.rs     // resolve marketplace URL (flag / registry / env). PURE.
├── git.rs        // shallow clone via `git`; presence check; temp-dir guard
├── catalog.rs    // scan cloned tree; parse .steer; build CatalogEntry list. PURE-ish (reads fs)
├── tui.rs        // ratatui + crossterm multi-select; TTY-gated
├── conflict.rs   // detect conflicts; resolve via prompt / global flag; backup naming
└── install.rs    // orchestrator: resolve → clone → scan → select → conflict → copy → summary
```

**Data model:**
- `CatalogEntry { name: String, description: Option<String>, template_sets: Vec<String>, workflow_path: PathBuf }` — `template_sets` from the workflow's `@template` (usually one; modeled as a vec for future multi-template support).
- `ConflictPolicy { Skip | Overwrite | Backup | Ask }`.
- `InstallPlan { copies: Vec<CopyOp>, }` where `CopyOp { src: PathBuf, dst: PathBuf }`; conflicts resolved to a final policy before copying.

**End-to-end data flow** (the `install.rs` orchestrator):
1. **Resolve source** (`source.rs`): determine URL from
   `--marketplace <url|name>` → registry (`source.rs` parses
   `.steer/marketplaces.toml` then `~/.steer/marketplaces.toml`) →
   `STEER_MARKETPLACE_URL` → fatal pre-network error.
2. **Clone** (`git.rs`): `git clone --depth 1 [--ref <r>] <url> <tmp>` into a
   temp dir owned by a RAII `TempGuard` (Drop removes it) so cleanup is
   guaranteed on every exit path. Pre-flight that `git` is on `PATH`.
3. **Scan** (`catalog.rs`): resolve catalog base = first of `<root>/.steer` (if
   it has `workflows/` or `templates/`) else `<root>`; read `workflows/*.steer`
   flat; for each, parse with `steer_syntax::parse`, then
   `workflow_description` + `workflow_template` → `CatalogEntry`.
4. **Select** (`tui.rs` unless flags): if stdout is a TTY and no
   `--workflows`/`--all`, render the multi-select; otherwise (`--workflows a,b`
   or `--all`) compute the selection directly. Non-TTY with no selection flag →
   fatal error with guidance.
5. **Conflict-resolve** (`conflict.rs`): build the target path set; detect
   conflicts; resolve to a per-file policy (global flag, else interactive prompt
   on TTY defaulting to skip, else skip-all on non-TTY); back up via `.bak[.N]`.
6. **Copy** (`install.rs`): write each `.steer` → `.steer/workflows/` and each
   referenced template-set dir → `.steer/templates/<set>/` (deduped across the
   selection so a shared set copies once); create `.steer/` subdirs as needed;
   skip writes entirely under `--dry-run`.
7. **Summary + cleanup**: print installed/skipped/backed-up; `TempGuard` drop
   removes the clone; return the exit code.

**Purity boundary:** `source.rs`, `catalog.rs`'s parsing, and `conflict.rs`'s
policy logic are pure/easily-unit-testable; only `git.rs`, `tui.rs`, and the
filesystem-copy parts of `install.rs` touch the world. The runtime crates are
untouched — `steer instance *` behaves identically before and after.

## Decisions

### Decision: Package-manager framing (bounded exception to the principle)

**Choice:** `install` (and future `uninstall`) are steer's **package-manager
side**, by analogy to `cargo install` vs `cargo run`. The "never touch the
outside world" principle continues to govern the **runtime execution loop** only.
**Rationale:** the request inherently requires touching git/the terminal/the
filesystem; the loop that matters for steer's correctness guarantees (the
agent-driven `instance` loop) does none of that, so it stays pure. Framing it as
a distinct side makes the exception explicit rather than an erosion.
**Alternatives considered:** (a) render everything as instructions for the agent
to execute (most faithful to the principle, but there is no "interactive window
in the CLI" — rejected by the user); (b) hybrid (CLI does mechanical bits, agent
does selection) — rejected for worse UX.

### Decision: New crate `steer-marketplace`

**Choice:** put all install code in a new crate rather than as modules in
`steer-cli`. **Rationale:** isolates the heavy world-touching deps
(`ratatui`/`crossterm`/`toml`) and the conceptual "package-manager side" behind a
clean boundary; `steer-core`/`steer-syntax` remain dependency-light and pure;
the new crate is independently unit-testable. **Alternatives considered:** modules
inside `steer-cli` (simpler, but pollutes the lean binary, mixes concerns, and
hides the boundary); a heavier split into `steer-tui` + `steer-marketplace`
(premature; one crate is enough for v1).

### Decision: Clone by shelling out to system `git`

**Choice:** `std::process::Command` running `git clone --depth 1 [--ref] <url>
<tmp>`. **Rationale:** zero new Rust deps; auth/SSH/HTTPS delegated to git's own
credential helpers and config; robust and well-understood; matches the user's
"clone" mental model. **Alternatives considered:** `git2`/libgit2 (bundled, but a
heavy native build dep and harder HTTPS/token handling); HTTP tarball via
`reqwest`+`tar`+`flate2` (no git needed, but heavier Rust deps and fragile
GitHub API/rate-limit/auth handling).

### Decision: ratatui + crossterm for the selection UI

**Choice:** a ratatui multi-select list driven by crossterm input.
**Rationale:** directly delivers the requested scrollable window with
space-toggle / enter-confirm / arrow-scroll; cross-platform; well-maintained.
**Alternatives considered:** hand-rolled TUI on crossterm alone (lighter, but
reinvents scrolling/multi-select and is less polished); a numbered stdin menu
(simplest and agent-friendly, but not the "interactive window" the user asked
for). Conflict resolution in v1 uses a simpler inline crossterm prompt rather
than full TUI screens — kept minimal; full-TUI conflict views are a future
enhancement.

### Decision: Marketplace repo structure mirrors `.steer/`, no manifest

**Choice:** the marketplace is a git repo with `workflows/*.steer` (each with
`@description`) and `templates/<set>/*.j2.md`, mirroring `.steer/`. The installer
scans the tree and reads `@description`/`@template` directly — **no manifest
file**. Templates for a workflow are resolved from its `@template` directive, so
there is nothing extra to keep in sync. **Rationale:** any existing steer project
pushed to git is automatically a valid marketplace; reuses the existing
`workflow_description` parser; no manifest-format design/maintenance burden.
**Alternatives considered:** a root `marketplace.toml`/`.json` manifest listing
packages (more metadata, but a second source of truth to drift from the tree);
per-package directories each with their own manifest (more structure than v1
needs).

### Decision: Catalog base resolves `<root>/.steer` then `<root>`

**Choice:** the catalog base is `<root>/.steer` when it contains a `workflows/`
or `templates/` directory, else `<root>`. **Rationale:** makes the claim "any
steer project is a marketplace" literally true (steer keeps its tree under
`.steer/`), while still supporting repos that place `workflows/`/`templates/` at
the root. **Alternatives considered:** `<root>` only (simpler, but rejects
`.steer`-rooted repos like steer itself); `<root>/.steer` only (rejects
root-level marketplace repos).

### Decision: Conflict default is skip; global flags for non-interactive

**Choice:** the interactive per-conflict prompt defaults to **skip** on a bare
enter; `--force`/`--skip`/`--backup` set a global policy with no prompts; non-TTY
without a flag also defaults to skip and reports. **Rationale:** a package
manager must never silently destroy a user's existing file; skip is the safe
default, and the global flags make the safe/explicit behavior scriptable.
**Alternatives considered:** default overwrite (dangerous); abort the whole
install on any conflict (too rigid); default to backup (clutters `.steer/` with
`.bak` files unexpectedly).

### Decision: Named registry is TOML, project-local overriding user-global

**Choice:** `.steer/marketplaces.toml` (project) and `~/.steer/marketplaces.toml`
(user), TOML `[marketplaces.<name>] url = "..."`; project wins on name clash;
home dir via `HOME`/`USERPROFILE`. **Rationale:** TOML is human-friendly for
hand-edited config; project-over-user mirrors how steer is project-scoped.
**Alternatives considered:** JSON (already-have-serde_json, but ugly to hand-edit);
env-var-only (rejected by the user); a `dirs`-crate-based global path (adds a dep
for ~3 lines of home resolution).

### Decision: Temp dir via a hand-rolled Drop guard, no `tempfile` crate

**Choice:** `std::env::temp_dir().join("steer-marketplace-<pid>")` wrapped in a
RAII guard whose `Drop` removes the tree. **Rationale:** guaranteed cleanup on
every exit path without a new dependency; a short-lived CLI makes PID-based
naming sufficient. **Alternatives considered:** the `tempfile` crate (idiomatic,
collision-resistant, but another dep for little gain here).

### Decision: TTY detection via `std::io::IsTerminal`; non-TTY requires flags

**Choice:** use `std::io::IsTerminal` (stable since 1.70, no dep) on stdout; when
non-interactive and no `--workflows`/`--all` is given, error with guidance rather
than open a UI. **Rationale:** prevents the CLI from blocking when driven by an
agent/CI; the std trait removes the need for an `atty`/`is-terminal` crate.
**Alternatives considered:** default to `--all` on non-TTY (convenient but
surprising and potentially destructive); always try the TUI (hangs non-TTY).

### Decision: Read `@template` via a new public `workflow_template` helper

**Choice:** add `pub fn workflow_template(&Module) -> Option<String>` to
`steer-core`, mirroring `workflow_description`, and re-export it. **Rationale:**
consistent with the existing one-directive-one-helper pattern; keeps
`extract_meta` private; the installer needs only the template-set name.
**Alternatives considered:** make `extract_meta`/`WorkflowMeta` public (exposes
more than needed); re-parse `@template` inside the new crate (duplicates logic).

## Risks / Trade-offs

- [**Security: a malicious marketplace workflow is later executed by the agent**]
  → Mitigation: install is an explicit user action against a user-chosen source;
  the catalog shows each workflow's `@description` before selection; the trust
  model is documented in the README. Signature/trust verification is a stated
  v1 non-goal, explicitly tracked for a future change.

- [**Principle erosion: a precedent for steer touching the world**]
  → Mitigation: all world-touching code lives in `steer-marketplace`; the
  `instance` runtime loop and `steer-core`/`steer-syntax` remain pure and
  dependency-light; the boundary is documented here and in the README. Future
  world-touching features must extend the package-manager side, not the runtime.

- [**Dependency bloat in a previously lean workspace**] → Mitigation: the three
  new deps are isolated in `steer-marketplace` (not pulled by `steer-core`);
  `ratatui`, `crossterm`, and `toml` are widely-used, pure-Rust, well-maintained.

- [**Non-TTY hang**] → Mitigation: explicit `IsTerminal` check; non-TTY without
  `--workflows`/`--all` is a fatal error with guidance; unit/integration tests
  cover the non-TTY path.

- [**`git` missing or clone fails mid-run**] → Mitigation: pre-flight `git`
  presence check; clear, located error messages; `TempGuard` Drop cleans the temp
  dir on every exit path, including failure.

- [**Backup name collisions**] → Mitigation: `.bak`, then `.bak.1`, `.bak.2`, …
  so no prior backup is ever clobbered.

- [**Registry typo silently disables a marketplace**] → Mitigation: a malformed
  `marketplaces.toml` is a fatal error naming the file; a missing file is not.

- [**Shared template set copied repeatedly**] → Mitigation: dedupe template sets
  across the whole selection and copy each set exactly once.

- [**Cross-platform (Windows) path/home/git differences**] → Mitigation: use
  `std::path`, `HOME`/`USERPROFILE`, and git's own cross-platform behavior; v1
  targets Linux (steer's primary platform), with Windows as best-effort and
  tracked for testing.

## Migration Plan

No migration needed. This change adds new capabilities without modifying existing
interfaces:

- **New crate** `crates/steer-marketplace` (additive; nothing depends on it yet
  except `steer-cli`).
- **`steer-cli`**: one new `WorkflowAction::Install` variant + a thin
  `run_install` delegating to the new crate. Existing subcommands are unchanged.
- **`steer-core`**: one new public function `workflow_template(&Module) ->
  Option<String>` + re-export — purely additive, non-breaking for existing
  consumers.
- **No changes** to `steer-syntax`, the instance runtime, validation, simulation,
  `list`, path resolution, or any existing spec's behavior.

Rollback (if needed): remove the `Install` variant from `steer-cli`, drop the
`steer-marketplace` crate and the `workflow_template` helper. Nothing else
references them, so removal is clean.

## Requirement coverage

Every spec requirement is satisfied by the architecture above:

| Capability | Requirement | Handled by |
| --- | --- | --- |
| marketplace-resolution | direct URL wins | `source.rs` URL-shape test |
| marketplace-resolution | named `--marketplace` via registry | `source.rs` registry lookup (project→global) |
| marketplace-resolution | env var default | `source.rs` |
| marketplace-resolution | no source → pre-network error | `source.rs` |
| marketplace-resolution | TOML registry format & error rules | `source.rs` + `toml` |
| workflow-install | shallow clone to temp + cleanup | `git.rs` + `TempGuard` |
| workflow-install | missing git / failed clone → fatal | `git.rs` pre-flight + errors |
| workflow-install | catalog scan + base resolution | `catalog.rs` |
| workflow-install | workflow + its `@template` sets | `catalog.rs` + `install.rs` copy |
| workflow-install | create `.steer/` structure | `install.rs` |
| workflow-install | `--workflows` / `--all` | `install.rs` selection |
| workflow-install | `--dry-run` | `install.rs` (no writes) |
| workflow-install | summary + exit codes | `install.rs` |
| interactive-install-selection | TTY-gated UI / non-TTY guidance | `tui.rs` + `IsTerminal` |
| interactive-install-selection | row content (templates shown) | `tui.rs` |
| interactive-install-selection | keybindings | `tui.rs` (ratatui+crossterm) |
| interactive-install-selection | empty-confirm no-op | `tui.rs` / `install.rs` |
| install-conflict-resolution | conflict detection | `conflict.rs` |
| install-conflict-resolution | interactive prompt, default skip | `conflict.rs` |
| install-conflict-resolution | `--force`/`--skip`/`--backup` | `conflict.rs` global policy |
| install-conflict-resolution | non-TTY defaults to skip | `conflict.rs` |
| install-conflict-resolution | `.bak[.N]` naming | `conflict.rs` |
