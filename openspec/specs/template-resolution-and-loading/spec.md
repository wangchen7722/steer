# Template Resolution And Loading

## Purpose

Define how steer resolves a callee name (for example `task`, `gather`, or `rca`) to a
`NodeTemplate` through a four-tier precedence-ordered search: the active workflow template
directory, the default directory, built-in fallback constants, and a generic task-like
fallback, plus `.j2.md` suffix discovery and per-process caching of the default directory.

## Requirements

### Requirement: Resolution follows a four-tier precedence

The resolver SHALL resolve a callee to a NodeTemplate using the following precedence in
order: first, a file in the active workflow template directory
`.steer/templates/<template_dir>/<callee>.j2.md` when a non-default `template_dir` is set;
second, a file in `.steer/templates/default/<callee>.j2.md`; third, a hardcoded built-in
fallback constant for the six default node names (task, ask, command, collect, print, judge);
fourth, a generic task-like NodeTemplate (a minimal required instruction string parameter
plus the built-in task body). Evidenced by `resolve_template_with_meta` and `fallback_template`
in `crates/steer-core/src/template.rs`. This four-tier chain is the ground truth even though
[docs/specs/templates.md](../../../../docs/specs/templates.md) lists only three tiers (the
generic unknown-callee fallback is the undocumented fourth tier).

#### Scenario: an unknown callee degrades to the generic task template

- **WHEN** a workflow calls a callee for which no template file and no built-in fallback
  exists
- **THEN** resolution returns the generic task-like NodeTemplate, which renders but carries
  no return specification and therefore emits no `steer instance set` prompt.

#### Scenario: a built-in fallback serves a default callee when no file is present

- **WHEN** the default template directory is absent or empty and a callee is one of the six
  built-in node names
- **THEN** resolution returns the hardcoded built-in fallback NodeTemplate for that name.

### Requirement: A missing file is silent degradation, never an error

Resolution SHALL NOT raise an error when a template file is absent or unreadable. An unknown
callee SHALL silently degrade to the generic task-like template, and a read failure for any
file SHALL be silently skipped. Evidenced by `resolve_template_with_meta` returning the
fallback via `unwrap_or` and `read_templates_dir` ignoring `read_dir`/`read_to_string`
errors in `crates/steer-core/src/template.rs`, and the test
`render_unknown_callee_falls_back_to_value_template`.

#### Scenario: a bare unknown callee renders without error

- **WHEN** a workflow calls an unknown callee with a positional instruction
- **THEN** the call renders an instruction using the generic task body and produces no error.

### Requirement: Directory discovery is keyed by stem with the j2.md suffix

When scanning a template directory the loader SHALL discover files by the `.j2.md` suffix and
key each parsed NodeTemplate by its stem (the suffix stripped). Files that do not end in
`.j2.md`, and entries whose contents cannot be read, SHALL be silently skipped. Evidenced by
`read_templates_dir` and `node_templates` stripping `.j2.md` via `strip_suffix` in
`crates/steer-core/src/template.rs`.

#### Scenario: a non-j2md file is ignored

- **WHEN** a template directory contains a `README.md` alongside `task.j2.md`
- **THEN** only `task` is keyed in the parsed map; the README is ignored.

### Requirement: None and Some(default) are equivalent and skip the workflow dir

A `template_dir` of `None` and a `template_dir` of `Some("default")` SHALL be equivalent:
both resolve from the `default/` directory. An explicit `Some("default")` SHALL skip the
workflow-directory lookup and SHALL NOT query a `default/default/` path, because `default`
is a reserved identity directory name. Only a non-`default` `template_dir` SHALL cause the
workflow-directory lookup to run first. Evidenced by the `if dir != "default"` guard in
`resolve_template_with_meta` in `crates/steer-core/src/template.rs`.

#### Scenario: an explicit default dir does not double-nest

- **WHEN** the active template_dir is `Some("default")`
- **THEN** resolution queries `.steer/templates/default/<callee>.j2.md` and never
  `.steer/templates/default/default/<callee>.j2.md`.

### Requirement: The default directory is cached once per process

The `default/` directory scan SHALL be cached once per process via a process-wide
OnceLock, so repeated resolutions within one CLI invocation reuse the parsed map. Each CLI
invocation is a fresh process and SHALL re-read the directory. Workflow-specific (non-default)
directories SHALL be re-read on each resolution and SHALL NOT be cached. Evidenced by the
`static CACHE: OnceLock<...>` in `node_templates` and the uncached `workflow_node_templates`
in `crates/steer-core/src/template.rs`.

#### Scenario: the default dir is read once per CLI invocation

- **WHEN** multiple callees are resolved in a single process
- **THEN** the `default/` directory is scanned at most once and the parsed map is reused.

### Requirement: The active template dir comes from the template directive

The active `template_dir` used in resolution SHALL be the value extracted from the workflow's
top-level `@template` directive and persisted across resumes. Extraction of that directive at
parse/start time (including the empty-string clears, the default identity, and top-level-only
recognition) is specified by
[workflow-directive-extraction](openspec/specs/workflow-directive-extraction/spec.md);
this capability covers only its consumption at render time.

#### Scenario: a workflow template dir overrides the default

- **WHEN** the active template_dir names a directory containing a `task.j2.md`
- **THEN** that directory's task template is used in preference to the default one.
