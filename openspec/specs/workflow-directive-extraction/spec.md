# Workflow Directive Extraction

## Purpose

Defines how top-level `@template`, `@context`, and `@description` directives
are extracted at parse time into persisted runtime metadata and a
non-persisted catalog description, with empty rendered values normalized to
absent.

## Requirements

### Requirement: Only top-level directives are scanned

Directive extraction SHALL scan only top-level `Meta` statements of a workflow
module; directives nested inside functions, loops, or conditionals SHALL NOT
contribute to the extracted metadata. Evidenced by `extract_meta` and
`workflow_description` in `crates/steer-core/src/storage.rs`, which iterate
the module's top-level statements.

#### Scenario: a top-level directive is extracted

- **WHEN** a workflow's top level contains `@context = "..."`
- **THEN** the rendered value is extracted into metadata.

#### Scenario: a nested directive is ignored

- **WHEN** an `@template` or `@context` directive appears inside a function or
  loop body
- **THEN** it does not affect the extracted metadata.

### Requirement: Each directive's literal value is rendered and empty becomes None

Each directive's literal value SHALL be rendered via
`eval_literal(...).render()`. The `@template` directive SHALL set
`meta.template_dir` and the `@context` directive SHALL set `meta.context`,
both persisted in `context.json`. When a directive's rendered value is the
empty string, the corresponding field SHALL be `None` (meaning default for
template, and no banner for context). Evidenced by `extract_meta` and the
tests `start_returns_context_from_directive` and
`start_returns_none_without_context_directive`. The resolution of
`meta.template_dir` into a render directory is covered by the companion
[template-resolution-and-loading](openspec/specs/template-resolution-and-loading/spec.md)
capability.

#### Scenario: a non-empty context directive is persisted and surfaced

- **WHEN** a workflow declares `@context = "welcome"` and an instance is
  started
- **THEN** `meta.context` is `Some("welcome")`, persisted in `context.json`,
  and `start` returns it as the banner description.

#### Scenario: an empty context directive becomes None

- **WHEN** a workflow declares `@context = ""` (or omits it)
- **THEN** `meta.context` is `None` and `start` returns `Ok(None)`.

#### Scenario: an empty template directive means default

- **WHEN** a workflow declares `@template = ""`
- **THEN** `meta.template_dir` is `None`, selecting the default template
  directory at render time.

### Requirement: The description directive is not persisted

The `@description = "..."` directive SHALL be extracted only by
`workflow_description` for use by `steer workflow list`; it SHALL have no
runtime effect and SHALL NOT be persisted in `context.json`. An empty rendered
description SHALL yield `None`. Evidenced by `workflow_description` in
`crates/steer-core/src/storage.rs` and the `workflow_description_*` tests.

#### Scenario: a description appears in the workflow list

- **WHEN** a workflow declares `@description = "..."` (non-empty)
- **THEN** `steer workflow list` shows the rendered description.

#### Scenario: description is absent from context.json

- **WHEN** an instance is started for a workflow with `@description`
- **THEN** the rendered description is NOT present in `context.json`.

#### Scenario: an empty description yields None

- **WHEN** a workflow declares `@description = ""` (or omits it)
- **THEN** `workflow_description` returns `None`.

### Requirement: meta is forward-compatible via serde defaults

The `meta` field SHALL be `#[serde(default)]` in `Context`, and its subfields
`template_dir` and `context` SHALL each be `#[serde(default)]`, so that
pre-`meta` instance files and files missing individual subfields still load.
Evidenced by `struct WorkflowMeta` and `struct Context` in
`crates/steer-core/src/context.rs`. The persisted shape of `meta` is pinned by
[instance-persistence](openspec/specs/instance-persistence/spec.md).

#### Scenario: a context.json without meta loads

- **WHEN** `load_context` reads an older `context.json` with no `meta` key
- **THEN** it loads successfully with `meta` at its default.

#### Scenario: a meta object missing a subfield loads

- **WHEN** `load_context` reads a `meta` object missing `template_dir` or
  `context`
- **THEN** the missing subfield defaults to `None`.
