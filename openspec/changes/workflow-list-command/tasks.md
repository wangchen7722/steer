# Tasks — `workflow-list-command`

> Implementation checklist. Each task is small enough for one session and ordered
> by dependency. Reference `specs/workflow-listing/spec.md` for *what* and
> `design.md` for *how*. Concrete code, commands, and expected output live in
> `plan.md`.

## 1. Core: description extraction

- [x] 1.1 Add `pub fn workflow_description(&steer_syntax::Module) -> Option<String>` to `crates/steer-core/src/storage.rs`, co-located with `extract_meta`, mirroring its `eval_literal(value).render()` (empty → `None`) pattern for the `@description` key.
- [x] 1.2 Re-export `workflow_description` from `crates/steer-core/src/lib.rs`.
- [x] 1.3 Add unit tests in `storage.rs`: present description, absent → `None`, empty string → `None`.

## 2. CLI: `workflow list` subcommand

- [x] 2.1 Add `List { dir: Option<PathBuf> }` variant to `WorkflowAction` in `crates/steer-cli/src/main.rs` with a doc comment.
- [x] 2.2 Wire dispatch: `WorkflowAction::List { dir } => run_list(dir.as_deref())`.
- [x] 2.3 Implement `run_list(dir: Option<&Path>)`: default `.steer/workflows/`; flat scan of `*.steer`; name = file stem; parse + `workflow_description`; placeholders `(no description)` / `(unparseable)` / `(unreadable)`; `(no workflows in <dir>)` when empty; sort alphabetically; pad-and-print two columns; exit 0.

## 3. CLI integration tests

- [x] 3.1 `list_shows_workflows_with_descriptions` — two workflows (one with `@description`, one without); assert names, description text, and `(no description)` all appear.
- [x] 3.2 `list_honors_custom_dir` — pass an explicit `<dir>` and assert only that dir's workflows are listed.
- [x] 3.3 `list_missing_dir_reports_no_workflows` — no `.steer/workflows/`; assert `no workflows` notice and exit 0.
- [x] 3.4 `list_marks_unparseable_file` — a `.steer` with a syntax error is listed with `(unparseable)`.

## 4. Catalog seeding + docs

- [x] 4.1 Add a concise `@description` line to `.steer/workflows/openspec-propose.steer`, `openspec-apply.steer`, and `os-bugfix.steer`.
- [x] 4.2 Update `README.md`: add `steer workflow list [dir]` to the CLI table and mention `@description` in the language/metadata section.
- [x] 4.3 Add `docs/specs/workflow-listing.md` (BDD scenarios) and update `docs/specs/cli.md` (subcommand recognition includes `list`).
- [x] 4.4 Update `docs/specs/index.md` to list the new `workflow-listing.md` (docs index convention).

## 5. Verification

- [x] 5.1 `cargo fmt --all -- --check` is clean.
- [x] 5.2 `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
- [x] 5.3 `cargo test --workspace --all-features` passes.
- [x] 5.4 Manual smoke test: `steer workflow list` in the repo root shows the three shipped workflows with their descriptions.
