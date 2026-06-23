# Design — `workflow-list-command`

## Context

steer is a workflow interpreter. Workflows live as `.steer` files, and the
shipped/author catalog is `.steer/workflows/`. The discovery layer
(`crates/steer-cli/src/main.rs::resolve_workflow`) already treats that directory
as the registry — it falls back to a flat lookup there when a `<workflow>` path
arg is not a file. What is missing is a way to **enumerate** the registry and see
what each workflow does at a glance.

Workflow-level metadata is expressed with `@`-directives (`@template`,
`@context`), parsed as `Stmt::Meta` and extracted in three near-identical spots
(`storage::extract_meta`, `simulate::apply_static_meta`, `vm::apply_meta`), each
using `eval_literal(value).render()` with empty → `None`. `@context` is
runtime-facing (shown by `start`/`status`); there is no listing-oriented
directive. Constraints: the change must be additive (no breaking behavior), must
not crash on bad catalog entries, and must keep the runtime path untouched by a
value that has no runtime consumer.

Stakeholders: external coding agents that drive steer (they pick workflows), and
human authors browsing the catalog.

## Goals / Non-Goals

**Goals:**

- `steer workflow list [dir]` enumerates `*.steer` files in a directory (default
  `.steer/workflows/`), sorted, and prints each name with its `@description`.
- Introduce `@description = "..."` as an optional, listing-oriented directive.
- Degrade gracefully: missing description, unparseable file, and missing/empty
  directory all produce helpful, non-crashing output.

**Non-Goals:**

- Surfacing `@description` in `instance start`/`status` output (runtime use).
  Flagged for a future change if wanted.
- Enforcing that `@description` is a string literal (any literal renders; type
  policing is out of scope, matching `@context`).
- Machine-readable output (`--json`).
- Recursive directory scanning (the catalog is flat, like `resolve_workflow`).
- Refactoring the three existing meta-extraction sites into one (out of scope;
  this change avoids all three).

## Architecture

Two small, isolated additions — one in the core library, one in the CLI — plus
docs and catalog seeding. No new module is strictly required.

```
crates/steer-core/src/storage.rs
   + pub fn workflow_description(&Module) -> Option<String>   // pure; mirrors extract_meta
crates/steer-core/src/lib.rs
   + pub use storage::workflow_description;                    // re-export
crates/steer-cli/src/main.rs
   + WorkflowAction::List { dir: Option<PathBuf> }             // clap subcommand
   + fn run_list(dir: Option<&Path>) -> ExitCode               // scan + parse + print
```

**Data flow for `list`:**

1. Resolve target dir (`dir` arg, else `.steer/workflows/`).
2. `read_dir`; on error or no entries → print `(no workflows in <dir>)`, exit 0.
3. For each entry whose extension is `steer`: take the file stem as the **name**;
   read the file; parse it; on success call `workflow_description(&module)` →
   description text or `(no description)`; on parse/read failure use an
   `(unparseable)` / `(unreadable)` marker. Push `(name, description)` to a row
   list. Non-`.steer` entries are skipped.
4. Sort rows by name; pad names to the longest; print `name  description`.

The CLI does the filesystem work (consistent with `run_validate`/`run_simulate`,
which call `load_workflow` then a pure `steer_core::*` function). The core crate
contributes only the pure extractor, consistent with its "no outside-world side
effects for workflow execution" stance — reading the tool's own catalog in the
CLI is bookkeeping, not workflow execution.

## Decisions

### Decision: dedicated `workflow_description` extractor (Option B), not a `WorkflowMeta` field

**Choice:** Add a standalone pure `pub fn workflow_description(&Module) ->
Option<String>`, used only by `list`. Do **not** add `description` to
`WorkflowMeta` and do **not** touch `storage::extract_meta`,
`simulate::apply_static_meta`, or `vm::apply_meta`.

**Rationale:** `@description` has no runtime consumer. Putting it in
`WorkflowMeta` would serialize an unused `Option<String>` into every
`context.json` and would pressure us to add a fourth copy of the
template/context extraction branch (or leave the struct inconsistently
populated). A single-purpose function keeps the runtime path byte-for-byte
unchanged, is trivially testable, and answers exactly one question.

**Alternatives considered:**

- _Option A — extend `WorkflowMeta` and route through all three extraction sites._
  More "uniform" (every `@`-directive lives in one struct), and would make a
  future `status shows description` feature cheaper. Rejected as YAGNI for this
  change; the uniformity buys nothing today and touches three files for a value
  none of them reads. Easy to promote later if a runtime consumer appears.

### Decision: extraction mirrors the `@context`/`@template` pattern

**Choice:** `let rendered = eval_literal(value).render(); if rendered.is_empty() { None } else { Some(rendered) }`.

**Rationale:** Identical semantics to the existing directives — empty is absent,
and a string with `{x}` interpolation degrades to a `{x}` placeholder under
static evaluation (no runtime scope at listing time). Consistency with
`@context`/`@template` is worth more than special-casing.

### Decision: name = file stem; flat, sorted enumeration

**Choice:** Name is `file_stem`; only top-level `*.steer` files; sort
alphabetically by name.

**Rationale:** The stem is the token `instance start`/`validate`/`simulate`
accept, so the listed name is actionable. Flat scan matches `resolve_workflow`'s
flat lookup (the catalog has no nested workflows). Sorting gives stable,
diff-friendly output.

### Decision: optional `[dir]` argument, default `.steer/workflows/`

**Choice:** `WorkflowAction::List { dir: Option<PathBuf> }`; `None` ⇒
`.steer/workflows/`.

**Rationale:** The user asked for "默认" (default) listing of `.steer/workflows/`.
An optional override is cheap, useful for tests and alternate catalogs, and
honors the "default" framing without forcing it.

### Decision: graceful placeholders, exit 0 on empty

**Choice:** Absent description → `(no description)`; unparseable →
`(unparseable)`; unreadable → `(unreadable)`; missing/empty dir →
`(no workflows in <dir>)`; all exit 0.

**Rationale:** `list` must never crash on a bad entry, and an empty/missing
catalog is a normal state (not an error). Listing a broken file by name with a
marker makes it discoverable instead of silently dropped.

## Risks / Trade-offs

- **[Risk] Parsing every file on each `list` could be slow for a huge catalog.**
  → Mitigation: at expected scale (handfuls of small files) the cost is
  negligible; no caching needed. If catalogs ever grow large, cache keyed on
  file mtime later.
- **[Risk] `@description` with interpolation renders a `{x}` placeholder.** →
  Mitigation: document that `@description` is a plain literal (mirrors `@context`
  static behavior); no enforcement, matching existing directives.
- **[Risk] Listing a parse-broken file could confuse users.** → Mitigation: the
  `(unparseable)` marker makes the breakage visible; `validate` remains the tool
  to diagnose it.
- **[Trade-off] Optional `[dir]` adds a sliver of CLI surface.** → Accepted; it
  is one optional positional and pays for itself in testability.

## Migration Plan

No migration needed. This change is purely additive: a new subcommand, a new
optional directive that existing extraction sites already ignore, and new docs.
No existing interface, behavior, file format, or `context.json` shape changes, so
there is nothing for consumers to update and no rollback concern beyond reverting
the commit.
