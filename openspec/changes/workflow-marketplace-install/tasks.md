# Tasks — `workflow-marketplace-install`

> Implementation checklist. Reference [`specs/`](./specs/) for what to build and
> [`design.md`](./design.md) for how. Each task is ordered by dependency and is
> verifiable with the stated completion criteria. The apply phase parses `- [ ]`.

## 1. Foundation: crate scaffold and `steer-core` helper

- [ ] 1.1 Create `crates/steer-marketplace` (Cargo.toml with deps `ratatui`, `crossterm`, `toml`, plus `steer-syntax`/`steer-core`; add to workspace `members`; `[lints] workspace = true`). **Done when** `cargo build -p steer-marketplace` succeeds and `cargo clippy -p steer-marketplace -- -D warnings` is clean.
- [ ] 1.2 Add `pub fn workflow_template(&Module) -> Option<String>` to `steer-core/src/storage.rs` (mirror `workflow_description`; empty rendered text → `None`) and re-export from `steer-core/src/lib.rs`. **Done when** unit tests cover present / empty / absent and pass.
- [ ] 1.3 Define the shared data model in `steer-marketplace/src/lib.rs`: `CatalogEntry { name, description, template_sets, workflow_path }`, `ConflictPolicy { Skip | Overwrite | Backup | Ask }`, `CopyOp`, `InstallArgs`. **Done when** it compiles and each type has a doc comment.

## 2. Source resolution (`source.rs`)

- [ ] 2.1 Implement direct-URL detection: a `--marketplace` value containing `://` or ending in `.git` is used verbatim. **Done when** unit test covers https and `*.git` (incl. SSH `git@...:.git`).
- [ ] 2.2 Implement registry parsing: read `.steer/marketplaces.toml` (project) then `~/.steer/marketplaces.toml` (user, via `HOME`/`USERPROFILE`), `[marketplaces.<name>] url`; project overrides user on clash. **Done when** unit tests cover local-hit, global-fallback, and local-overrides-global using temp files.
- [ ] 2.3 Implement precedence: `--marketplace <url>` > `--marketplace <name>` (registry) > `STEER_MARKETPLACE_URL` > fatal pre-network error with guidance. **Done when** unit tests pin each precedence tier and the no-source error returns a descriptive `Err` without touching the network.
- [ ] 2.4 Implement registry error rules: missing file is not an error; a present-but-malformed file is a fatal error naming the file. **Done when** unit tests cover both.

## 3. Git clone (`git.rs`)

- [ ] 3.1 Implement `TempGuard`: creates `std::env::temp_dir().join("steer-marketplace-<pid>")`, removes the tree on `Drop`. **Done when** a test creates one, drops it, and asserts the dir is gone.
- [ ] 3.2 Implement `git` presence pre-flight (`git --version`); clear error if missing. **Done when** a test asserts the error path when `git` is absent (PATH stub).
- [ ] 3.3 Implement `git clone --depth 1 [--ref <r>] <url> <tmp>` with error propagation and guaranteed cleanup on failure. **Done when** an integration test clones a tiny local fixture repo (use a `file://`/temp-origin) into a `TempGuard` and asserts the catalog files are present, then gone after drop.

## 4. Catalog scan (`catalog.rs`)

- [ ] 4.1 Implement catalog-base resolution: `<root>/.steer` if it contains `workflows/` or `templates/`, else `<root>`. **Done when** unit tests cover both layouts.
- [ ] 4.2 Scan `<base>/workflows/*.steer` (flat); parse each with `steer_syntax::parse`; build `CatalogEntry` (name=stem; description via `workflow_description`; `template_sets` via `workflow_template`; unparseable file → listed with an `(unparseable)` marker, mirroring `list`). **Done when** unit test on a fixture tree yields the expected entries.
- [ ] 4.3 Return an empty catalog (with no error) when no `.steer` files exist. **Done when** unit test on an empty base returns `vec![]`.

## 5. Conflict resolution (`conflict.rs`)

- [ ] 5.1 Implement conflict detection: compute the target path set for selected entries (each `.steer/workflows/<name>.steer` + every file under each referenced `.steer/templates/<set>/`) and flag existing paths. **Done when** unit test detects workflow- and template-set conflicts.
- [ ] 5.2 Implement the global policy flags (`--force`/`--skip`/`--backup`) and the `Ask` default. **Done when** unit tests map each flag to its policy.
- [ ] 5.3 Implement the interactive per-conflict prompt (`s`/`o`/`b` + `S`/`O`, default skip on bare enter) via crossterm; non-TTY without a global flag resolves every conflict as skip and reports them. **Done when** prompt-key handling is unit-tested and the non-TTY skip path is tested.
- [ ] 5.4 Implement backup naming: `.bak`, then `.bak.1`, `.bak.2`, … without clobbering. **Done when** unit tests cover first backup and disambiguation.

## 6. Selection UI (`tui.rs`)

- [ ] 6.1 Implement TTY detection (`std::io::IsTerminal` on stdout) and the show-UI-vs-require-flag decision (non-TTY + no `--workflows`/`--all` → fatal guidance error). **Done when** the decision logic is unit-testable (inject an `is_tty` bool).
- [ ] 6.2 Implement the ratatui multi-select view: scrollable rows showing checkbox + name + description (placeholder when absent) + the template set(s) the row would install. **Done when** it renders a fixture catalog without panicking (smoke test).
- [ ] 6.3 Implement keybindings: space toggle, enter confirm, up/down + `j`/`k` scroll, `a` select-all, `n` clear, `q`/`Esc` cancel. **Done when** key→action mapping is unit-tested.
- [ ] 6.4 Confirming with an empty selection installs nothing and exits 0. **Done when** a test of the confirm path with zero selections yields an empty plan.

## 7. Install orchestrator and CLI wiring (`install.rs`, `main.rs`)

- [ ] 7.1 Implement the orchestrator: resolve → clone → scan → select → conflict-resolve → copy (dedupe template sets across the selection; create `.steer/workflows/` and `.steer/templates/<set>/` as needed; warn on a referenced-but-missing template set) → summary → `TempGuard` cleanup. **Done when** an end-to-end test installs a workflow + its template set into a temp project.
- [ ] 7.2 Implement `--dry-run`: resolve/clone/scan/select/conflict-plan run, but no file is written/removed; print the planned actions. **Done when** a test leaves `.steer/` untouched and prints the plan.
- [ ] 7.3 Implement `--workflows a,b,c` and `--all` (skip the TUI); an unknown name is a fatal error that installs nothing. **Done when** tests cover subset install, all install, and the unknown-name error.
- [ ] 7.4 Implement the summary output and exit codes: 0 for success / cancel / empty catalog; non-zero for fatal errors. **Done when** tests assert the exit code per path.
- [ ] 7.5 Wire `WorkflowAction::Install { marketplace, workflows, all, force, skip, backup, dry_run, ref_ }` in `crates/steer-cli/src/main.rs` with a thin `run_install` delegating to `steer_marketplace::install`. **Done when** `steer workflow install --help` lists all flags and a manual run works.

## 8. Documentation

- [ ] 8.1 Update `README.md` CLI section: the new `steer workflow install` command, all flags, the marketplace repo layout (mirrors `.steer/`), `STEER_MARKETPLACE_URL`, `.steer/marketplaces.toml`, and the package-manager/trust note. **Done when** the README accurately reflects the shipped behavior.
- [ ] 8.2 Add behavior specs under `docs/specs/` for the install command and update `docs/specs/index.md` (and any parent index) per the docs index convention. **Done when** indexes list every new/changed file and the index tree is consistent.
- [ ] 8.3 Run the full gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`. **Done when** all three pass clean.
