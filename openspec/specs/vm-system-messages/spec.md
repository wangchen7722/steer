# VM System Messages

## Purpose

Define the system messages steer emits itself (the check report, the retry context, and the
instance start and status output), which use a single-brace placeholder syntax distinct from
the workflow templates' double-brace Jinja2 syntax, the exact checked report command strings,
and the context-line and program-counter suppression rules that shape agent-facing output.

## Requirements

### Requirement: System messages use single-brace placeholders, not Jinja2

The VM system messages SHALL use a single-brace placeholder syntax (a name enclosed in one
pair of braces) rendered by direct string replacement, and SHALL NOT use the double-brace
Jinja2 syntax used by workflow templates. The two placeholder syntaxes SHALL coexist without
being mixed: a single brace in an instruction body is a literal, while a single brace in a
system message template is a substitution site. Evidenced by the
`{name}`/`{instance}`/`{reason}` placeholders and the `str::replace`-based render functions
in `crates/steer-core/src/template.rs`.

#### Scenario: a single-brace placeholder is substituted in a system message

- **WHEN** a system message template contains a single-brace placeholder and is rendered
- **THEN** the placeholder is replaced by its value via string replacement.

#### Scenario: a double-brace token is not interpreted in a system message

- **WHEN** a system message is rendered
- **THEN** no Jinja2 double-brace evaluation occurs; only single-brace placeholders are
  substituted.

### Requirement: The report section appends two exact checked commands

The VM SHALL auto-append a report section to every check instruction that emits exactly two
reporting forms. The passed form SHALL be the command setting the checked value to the JSON
object with passed true, and the failed form SHALL be the command setting the checked value to
the JSON object with passed false and a reason field. Both JSON values SHALL be wrapped in
single quotes as shell quoting. The report section SHALL NEVER be part of a template's on_check
body; it is always appended by the VM. Evidenced by `CHECK_REPORT_TEMPLATE` and
`render_check_report` in `crates/steer-core/src/template.rs`, consumed in the check gate
described by [runtime-check-gate](openspec/specs/runtime-check-gate/spec.md).

#### Scenario: the passed form sets checked to passed true

- **WHEN** the report section is rendered for an instance
- **THEN** it contains the command setting checked to single-quoted JSON with passed true.

#### Scenario: the failed form sets checked to passed false with a reason

- **WHEN** the report section is rendered for an instance
- **THEN** it contains the command setting checked to single-quoted JSON with passed false and
  a reason field.

### Requirement: The retry context cites the failure reason and retry number

The VM SHALL prefix a re-issued instruction after a check failure with a retry context that
names the retry attempt by a one-based count and cites the failure reason. The retry context
SHALL be built from a retry-count placeholder and a reason placeholder via string replacement.
Evidenced by `RETRY_CONTEXT_TEMPLATE` and `render_retry_context` in
`crates/steer-core/src/template.rs`, and the test `render_retry_context_format`.

#### Scenario: the retry context includes the one-based retry number

- **WHEN** a retry context is rendered for the first retry
- **THEN** the text contains the retry marker with the number one.

#### Scenario: the retry context includes the failure reason

- **WHEN** a retry context is rendered with a given reason
- **THEN** the text contains that failure reason.

### Requirement: Start and status append the context description as a second line

The instance start and status output SHALL append the workflow context description, when
present, after the status line as a second line, and SHALL omit it entirely when no context
description is present. The status line SHALL be of the form naming the instance and its
status. Evidenced by `render_start_output` and `render_status_output` and their context
templates in `crates/steer-core/src/template.rs`, and the tests
`render_start_output_with_context` and `render_status_output_running_with_context`.

#### Scenario: a present context description is appended

- **WHEN** the workflow declares a context description and start or status is rendered
- **THEN** the description appears as a second line after the status line.

#### Scenario: no context description yields a single status line

- **WHEN** no context description is declared and start or status is rendered
- **THEN** the output is a single status line with no appended context.

### Requirement: Status output never exposes the program counter

The instance status output SHALL NOT expose the program counter (pc) in user-facing text. The
status line reports only the instance name and a status string, never an internal execution
position. Evidenced by `render_status_output` in `crates/steer-core/src/template.rs` and the
test `render_status_output_running_with_context` asserting pc does not appear.

#### Scenario: the program counter is absent from status output

- **WHEN** instance status output is rendered
- **THEN** the output contains no program-counter token.
