# Workflow Discovery and Listing

## Purpose

Pins how the `<workflow>` argument is resolved for `instance start`,
`workflow validate`, and `workflow simulate`, and how
`steer workflow list [dir]` enumerates `*.steer` files with their descriptions.
These resolution rules and the listing output are an external compatibility
surface relied on by authors and agents.

## Requirements

### Requirement: Workflow resolution consults the explicit path first

The CLI SHALL resolve the `<workflow>` argument the same way for `instance
start`, `workflow validate`, and `workflow simulate`. The path as given
(CWD-relative, absolute, or explicit relative) MUST be consulted first; if it is
a regular file, it MUST win. Evidenced by `resolve_workflow` in
`crates/steer-cli/src/main.rs` and the test
`explicit_cwd_file_takes_precedence_over_steer_workflows`.

#### Scenario: an explicit file path wins

- **WHEN** the user passes a path that is an existing regular file
- **THEN** that file is used as the workflow, even if a same-named file exists
  under `.steer/workflows/`.

#### Scenario: an explicit CWD file takes precedence over a broken namesake

- **WHEN** the given path is a readable file in the current directory and a
  same-named broken file exists under `.steer/workflows/`
- **THEN** the explicit current-directory file is used and no parse error from
  the namesake is reported.

### Requirement: A flat `.steer/workflows/` fallback resolves bare names

When the given path is not an existing regular file, the CLI SHALL fall back to a
flat (non-recursive) lookup under `.steer/workflows/` by file name. When the
given name has no extension, `.steer` MUST be auto-appended before the flat
lookup (e.g. a bare name resolves to `.steer/workflows/<name>.steer`). If nothing
matches, the original argument MUST be returned unchanged so the caller's
"cannot read" error stays backward-compatible. Evidenced by `resolve_workflow`
in `crates/steer-cli/src/main.rs` and the tests
`instance_start_discovers_workflow_under_steer_workflows`,
`validate_discovers_workflow_by_bare_name_without_extension`,
`validate_reports_cannot_read_when_workflow_is_nowhere_to_be_found`.

#### Scenario: a bare name discovers a workflow under `.steer/workflows/`

- **WHEN** the user passes a bare name with no extension and a matching
  `.steer/workflows/<name>.steer` file exists
- **THEN** that file is used as the workflow.

#### Scenario: an unresolvable name falls through unchanged

- **WHEN** the given name matches no explicit file and no flat-lookup candidate
- **THEN** the original argument is returned unchanged, producing a "cannot read"
  error against the original path.

### Requirement: `workflow list` enumerates `*.steer` files by stem, sorted and padded

`steer workflow list [dir]` SHALL enumerate `*.steer` files only (by extension),
listing each by its file stem (no `.steer` extension). Entries MUST be sorted by
name ascending, and each line MUST be `<name><padding><description>` with a
two-space separator and names left-padded to the longest name's width. The
default scan directory MUST be `.steer/workflows/`; an optional positional `<dir>`
MUST override it. Evidenced by `run_list` in `crates/steer-cli/src/main.rs` and
the tests `list_shows_workflows_with_descriptions`,
`list_honors_custom_dir`.

#### Scenario: workflows are listed sorted and padded

- **WHEN** the user runs `steer workflow list` in a directory containing
  `*.steer` files
- **THEN** each workflow's stem is printed with its description, sorted by stem
  ascending and padded so descriptions share a two-space-separated column.

#### Scenario: a custom directory is honored

- **WHEN** the user runs `steer workflow list <dir>` with a positional directory
- **THEN** that directory is scanned instead of the default `.steer/workflows/`.

#### Scenario: non-steer files are skipped

- **WHEN** the scan directory contains files without the `.steer` extension
- **THEN** those files are not listed.

### Requirement: Description resolution uses fixed placeholders for missing, unparseable, and unreadable files

For each listed workflow, the description SHALL resolve as: a present non-empty
`@description` directive prints its text; an absent or empty description prints
"(no description)"; an unparseable file prints "(unparseable)" and listing
continues without aborting; an unreadable file prints "(unreadable)". Evidenced
by `read_description` in `crates/steer-cli/src/main.rs` and the test
`list_marks_unparseable_file`.

Note: `docs/specs/workflow-listing.md` documents only "(no description)" and
"(unparseable)"; the "(unreadable)" placeholder is produced by `read_description`
for I/O failures and is the authoritative behavior (code wins over docs).

#### Scenario: a present description is printed

- **WHEN** a listed workflow declares a non-empty `@description`
- **THEN** that description text is printed alongside the workflow's stem.

#### Scenario: a missing description prints a placeholder

- **WHEN** a listed workflow has no `@description` directive (or it is empty)
- **THEN** "(no description)" is printed alongside the workflow's stem.

#### Scenario: an unparseable file is marked and skipped over

- **WHEN** a listed workflow file does not parse
- **THEN** "(unparseable)" is printed alongside its stem and listing continues
  with the remaining files (it does not abort).

#### Scenario: an unreadable file is marked

- **WHEN** a listed workflow file cannot be read
- **THEN** "(unreadable)" is printed alongside its stem.

### Requirement: A missing or empty directory is not an error

`steer workflow list` SHALL NOT fail when the scan directory is missing or empty.
stdout MUST be "(no workflows in <dir>)" and the exit code MUST be 0. Evidenced
by `run_list` in `crates/steer-cli/src/main.rs` and the tests
`list_missing_dir_reports_no_workflows`.

#### Scenario: a missing directory reports no workflows

- **WHEN** the user runs `steer workflow list <dir>` and `<dir>` does not exist
- **THEN** stdout contains "no workflows" and the exit code is 0.

#### Scenario: an empty directory reports no workflows

- **WHEN** the scan directory exists but contains no `*.steer` files
- **THEN** stdout is "(no workflows in <dir>)" and the exit code is 0.

### Requirement: `@description` is runtime-inert outside of `list`

The `@description` directive SHALL affect ONLY `steer workflow list`. It MUST be
runtime-inert for `validate`, `simulate`, `instance start`, and `instance
status`: it SHALL NOT influence validation results, simulated output, instance
creation, or status rendering. Evidenced by `read_description` being called only
from `run_list`, and by `steer_core::workflow_description` being used solely for
listing; cf. [cli-command-surface](openspec/specs/cli-command-surface/spec.md)
for the `@context` directive, which is the description-like directive that DOES
affect start/status output.

#### Scenario: description does not affect validation

- **WHEN** a workflow declares a `@description` and the user runs
  `steer workflow validate`
- **THEN** the description text does not appear in validation output and does not
  change the pass/fail result.

#### Scenario: description does not affect start or status output

- **WHEN** a workflow declares a `@description` and the user runs
  `steer instance start` or `steer instance status`
- **THEN** the description text does not appear in the start or status output
  (only `@context` does).
