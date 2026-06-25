# CLI Error Reporting

## Purpose

Pins the CLI-layer error-reporting convention so every failure is observable the
same way: human-facing errors go to stderr prefixed with `error:`, located
diagnostics carry source positions, and every error path exits FAILURE. This
keeps stdout clean for agents and scripts that parse command output.

## Requirements

### Requirement: All application errors go to stderr prefixed with `error:`

The CLI SHALL route every application-level error message to STDERR, prefixed
with the literal token "error: ". Error text SHALL NEVER appear on stdout.
Evidenced by the uniform eprintln calls across `run_instance_start`,
`with_instance_result`, `run_instance_status`, `load_workflow`, and
`run_validate` in `crates/steer-cli/src/main.rs`.

#### Scenario: an error is written to stderr only

- **WHEN** any documented failure path is hit
- **THEN** the diagnostic is written to stderr beginning with "error: " and
  stdout contains no error text.

#### Scenario: success writes to stdout only

- **WHEN** a command completes its happy path
- **THEN** its output goes to stdout and stderr is empty of error messages.

### Requirement: Located diagnostics carry a line and column

Parse and semantic diagnostics SHALL carry a source position computed via
`steer_syntax::line_col`, rendered as the suffix " (at line <n>, col <n>)"
appended to the error message. Evidenced by the `line_col` calls in
`load_workflow` and `run_validate` in `crates/steer-cli/src/main.rs` and the test
`validate_parse_error_exits_nonzero` asserting stderr contains "at line".

#### Scenario: a parse error is located

- **WHEN** a workflow file fails to parse
- **THEN** stderr contains "error: <message> (at line <n>, col <n>)".

#### Scenario: a semantic diagnostic is located

- **WHEN** a workflow file parses but fails a semantic check
- **THEN** each diagnostic on stderr carries " (at line <n>, col <n>)".

### Requirement: File-read failures report `cannot read` with the path and cause

When a workflow file cannot be read, the CLI SHALL print
"error: cannot read <path>: <io error>" to stderr, where `<path>` is the
resolved workflow path and `<io error>` is the underlying I/O error's Display.
Evidenced by the read-failure branches of `load_workflow` and
`run_instance_start` in `crates/steer-cli/src/main.rs` and the tests
`validate_missing_file_exits_nonzero`,
`validate_reports_cannot_read_when_workflow_is_nowhere_to_be_found`.

#### Scenario: a missing file reports cannot read

- **WHEN** the resolved workflow path does not exist
- **THEN** stderr contains "cannot read" with the path and the exit code is
  nonzero.

#### Scenario: an unresolvable name reports cannot read against the original

- **WHEN** the workflow argument matches no explicit file and no flat-lookup
  candidate (see [workflow-discovery-and-listing](openspec/specs/workflow-discovery-and-listing/spec.md))
- **THEN** the original argument is used in the "cannot read" message, keeping
  the error backward-compatible.

### Requirement: Invalid instance names report a fixed message

The CLI SHALL reject an invalid instance name with a diagnostic of the form
"error: invalid instance name" carrying the offending value, printed to stderr.
Evidenced by `instance_dir` in `crates/steer-cli/src/main.rs`, which returns the
"invalid instance name" message consumed by the `eprintln!("error: {e}")` sites.
The set of invalid names is normative in
[instance-name-validation](openspec/specs/instance-name-validation/spec.md) and
the command surface is pinned in
[cli-command-surface](openspec/specs/cli-command-surface/spec.md).

#### Scenario: an invalid instance name is reported

- **WHEN** the user runs an instance command with an invalid name
- **THEN** stderr reports "error: invalid instance name" with the offending
  value and the exit code is nonzero.

### Requirement: Every error path exits FAILURE

The CLI SHALL map every application error path to `ExitCode::FAILURE` (1). No
application error SHALL exit 0, and no application success SHALL exit nonzero.
Application logic SHALL emit only SUCCESS (0) or FAILURE (1); clap's own exit
code for unknown subcommands or argument-parse failures is an inherited part of
the external surface. Evidenced by the uniform `ExitCode::FAILURE` returns
across the error branches of `run_validate`, `run_simulate`, `run_instance_*`,
and `with_instance_result` in `crates/steer-cli/src/main.rs`.

#### Scenario: any error path exits nonzero

- **WHEN** any documented failure path is hit (unreadable file, parse error,
  semantic failure, invalid instance name, type/store error, or load failure)
- **THEN** the process exits with code 1.

#### Scenario: errors never exit 0

- **WHEN** an application error occurs
- **THEN** the exit code is nonzero, never 0.
