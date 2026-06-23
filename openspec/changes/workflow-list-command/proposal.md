# Proposal — `workflow-list-command`

## Why

steer ships workflows under `.steer/workflows/` and resolves a `<workflow>` path
argument by falling back to that directory, but there is **no way to see what
workflows exist**. To learn what `openspec-propose` or `os-bugfix` does today, an
agent or user must open each `.steer` file and read it. That friction matters
because steer is driven by external agents: an agent that can enumerate the
catalog and read a one-line summary per workflow can choose the right workflow
without speculative file reads.

`@context` already exists, but it is runtime-oriented prose shown by
`instance start` / `instance status`. Reusing it for a catalog listing would
overload its meaning and couple the listing format to the run-output format. We
need a dedicated, listing-oriented `@description` and a command that prints it.

## What Changes

- **New CLI subcommand** `steer workflow list [dir]` that enumerates `*.steer`
  files in a directory (default `.steer/workflows/`) and prints each workflow's
  name (file stem) alongside its `@description`, sorted alphabetically.
- **New `@description` directive** (`@description = "..."`) — a top-level
  workflow metadata line, parallel to `@template`/`@context`, whose sole consumer
  is `list`. It is optional and has no runtime effect.
- **Robust listing behavior**: a missing `@description` prints a
  `(no description)` placeholder; an unparseable file is still listed with an
  `(unparseable)` marker; a missing/empty directory prints
  `(no workflows in <dir>)` and exits successfully.
- **Catalog seeding**: the three shipped workflows (`openspec-propose`,
  `openspec-apply`, `os-bugfix`) gain an `@description` so `list` is useful
  immediately.
- **Docs**: the README CLI section and `docs/specs/` behavior specs document the
  new command and directive.

## Capabilities

### New Capabilities

- `workflow-listing`: enumerating the workflow catalog (`steer workflow list`)
  and the `@description` directive that annotates catalog entries.

### Modified Capabilities

_(None. `@description` is additive; discovery/path-resolution, validation,
simulation, and the instance runtime are unchanged. No existing requirement's
behavior changes.)_

## Impact

- **Code**
  - `crates/steer-cli/src/main.rs` — new `WorkflowAction::List { dir: Option<PathBuf> }`
    variant and a `run_list` handler (directory scan, parse, extract, print).
  - `crates/steer-core/src/storage.rs` — new pure
    `pub fn workflow_description(&Module) -> Option<String>`, co-located with
    `extract_meta` (same `eval_literal().render()` pattern).
  - `crates/steer-core/src/lib.rs` — re-export `workflow_description`.
- **Tests** — unit tests for `workflow_description` (present / absent / empty);
  CLI integration tests for `list` (descriptions shown, custom dir, missing dir,
  unparseable file).
- **Docs** — `README.md` (CLI table + `@description`); `docs/specs/cli.md`
  (subcommand recognition); new `docs/specs/workflow-listing.md` +
  `docs/specs/index.md` update (index convention).
- **Catalog** — `.steer/workflows/{openspec-propose,openspec-apply,os-bugfix}.steer`
  each gain an `@description` line.
- **APIs / dependencies / performance / security** — none. The change is purely
  additive; no new dependencies; scanning dozens of small files is trivial; no
  new I/O surface beyond reading the existing catalog directory.
