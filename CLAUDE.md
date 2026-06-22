# Project Guidelines

## Communication Language

- **Conversation and replies:** Respond in Chinese (Simplified).
- **Files and written materials:** Author all files, documents, code comments, commit messages, and other written artifacts in English.

## Git Commits

- Do **not** include the `Co-Authored-By` trailer (or any similar co-author attribution) in commit messages.

## Branch Naming

- Development for a feature or fix happens on a branch named **`<slug>-dev`**,
  where `<slug>` is the kebab-case identifier for the work (e.g. the bug slug
  used by the `os-bugfix` workflow, or a short feature name). Examples:
  `sensor-crash-on-boot-dev`, `bugfix-dev`.
- `master` is the integration branch. Do not commit in-progress feature work
  directly to `master`; branch first into `<slug>-dev`.

## Automated Checks (Rust)

Formatting and linting are automated checks, not review preferences.

Before submitting a change, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

During local development, use:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

The lint gate is clippy's **default** set (`clippy::all`), wired in through
`[workspace.lints]` + each crate's `[lints] workspace = true`. Pedantic,
`unwrap_used`, and `missing_docs` are intentionally not enabled — see the
per-crate `lib.rs` `#![allow(...)]` blocks for the project's stance. Detailed
Rust conventions live under [`docs/references/rust-code-style/`](./docs/references/rust-code-style/index.md).

## Project Reference Files

- **`docs/design.md`** — Design rationale for steer. Read this for *why* steer is
  designed the way it is (control-unit vs. execution-unit model, the "steer never
  touches the outside world" principle, language/task/template design). It is written
  in Chinese.
- **`README.md`** — How to use the steer **tool**: build/test commands, the `.steer`
  language syntax, the CLI surface, and the repo layout. Read this for *how to run and
  use* steer.
- **`docs/specs/`** — Behavior specifications (BDD, Given/When/Then) for the current
  tool, organized by implementation layer. A living record of implemented behavior and
  requirements so long-running work is not lost. Start at `docs/specs/index.md`.

When a task needs design context, consult `docs/design.md`; for CLI/syntax/usage,
consult `README.md`; for expected behavior of a feature, consult
`docs/specs/`.

## Documentation Index Convention (`docs/`)

To let the agent load context **progressively, level by level** instead of scanning
blind, the `docs/` tree is navigated through per-folder indexes:

- **Every folder under `docs/` (including `docs/` itself) MUST contain an `index.md`.**
  This applies recursively at every depth.
- Each `index.md` lists **every direct child** of that folder — both subfolders and
  files — and gives each a **one-line name + short description**. Nothing at that level
  is omitted; the index is the complete map of the folder.
- Subfolders are linked by relative path (e.g. `[references](references/index.md)` or
  `[references](references/)`), pointing to the child's own `index.md` so the agent can
  descend one level at a time.
- **Reading protocol for the agent:** start at `docs/index.md`, read it, then only
  descend into the specific child index that matches the task — never enumerate the whole
  tree up front. Context is pulled in on demand, level by level.
- **Authoring protocol:** whenever you add, remove, rename, or move a file or subfolder
  anywhere under `docs/`, update the `index.md` of **its parent folder** in the same
  change. Keep indexes authoritative — an index that disagrees with the directory is a
  bug.
