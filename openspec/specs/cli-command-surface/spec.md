# CLI Command Surface

## Purpose

Pins the complete externally observable `steer` command tree — exact subcommand
names, positional/flag arguments, stdout strings, and exit codes — as the
primary compatibility surface for `steer-cli`. Changing any pinned string is a
breaking change to every downstream agent and integration.

## Requirements

### Requirement: The top-level command tree has exactly two resources plus a version flag

The CLI SHALL expose exactly two top-level resources — `steer workflow <action>`
and `steer instance <action>` — and SHALL support a `--version`/`-V` flag that
prints the package version. No other top-level resource SHALL exist. Evidenced
by the `Resource` enum (`Workflow`, `Instance`) and the `#[command(version)]`
attribute on `Cli` in `crates/steer-cli/src/main.rs`.

#### Scenario: version flag prints the version

- **WHEN** the user runs `steer --version` (or `steer -V`)
- **THEN** the CLI prints the package version and exits successfully.

#### Scenario: an unknown top-level resource is rejected

- **WHEN** the user runs `steer <unknown>` for a word that is not `workflow`,
  `instance`, or a built-in flag
- **THEN** the CLI exits with a nonzero status (clap's standard usage error).

### Requirement: `workflow validate` prints the path with `OK` on success and nonzero on failure

`steer workflow validate <workflow>` (one positional `workflow: PathBuf`) SHALL
parse and semantic-validate the file. On success it SHALL print exactly
`<path>: OK` to stdout and exit 0. On a semantic error it SHALL print each
diagnostic to stderr as `error: <msg> (at line <n>, col <n>)` followed by a
summary line `<path>: <N> error(s)` and exit FAILURE. On a parse error it SHALL
print `error: <e> (at line <n>, col <n>)` to stderr and exit FAILURE. On an
unreadable or missing file it SHALL exit FAILURE. Evidenced by `run_validate`
in `crates/steer-cli/src/main.rs` and the tests
`validate_valid_file_exits_zero_and_reports_ok`,
`validate_value_task_without_return_exits_nonzero_with_message`,
`validate_parse_error_exits_nonzero`,
`validate_missing_file_exits_nonzero`.

#### Scenario: a valid workflow reports OK

- **WHEN** the user runs `steer workflow validate <workflow>` on a syntactically
  and semantically valid file
- **THEN** stdout contains `OK` (shape `<path>: OK`) and the exit code is 0.

#### Scenario: a semantic validation failure exits nonzero

- **WHEN** the workflow parses but fails a semantic check (e.g. a `task` with a
  `return=` annotation lacking a declared return type)
- **THEN** stderr carries the diagnostic and the exit code is nonzero.

#### Scenario: a parse error exits nonzero

- **WHEN** the workflow file does not parse
- **THEN** stderr contains `at line` and the exit code is nonzero.

#### Scenario: an unreadable file exits nonzero

- **WHEN** the workflow path does not exist or cannot be read
- **THEN** stderr contains `cannot read` and the exit code is nonzero.

### Requirement: `workflow simulate` renders steps and prints `(no action nodes)` when empty

`steer workflow simulate <workflow>` (one positional) SHALL render every action
instruction in source order. For each step it SHALL print a `[<i>] <callee>`
header line, then the rendered instruction body, then a blank line, numbering
steps from 1. With zero action nodes it SHALL print exactly `(no action nodes)`.
It SHALL exit 0 whenever the file parses; parse or load errors exit nonzero.
Evidenced by `run_simulate` in `crates/steer-cli/src/main.rs` and
[workflow-simulation](openspec/specs/workflow-simulation/spec.md).

#### Scenario: steps are numbered with a callee header

- **WHEN** the user runs `steer workflow simulate <workflow>` on a workflow with
  action nodes
- **THEN** each step prints as a `[<i>] <callee>` header, the rendered body, and
  a blank-line separator.

#### Scenario: an empty workflow reports no action nodes

- **WHEN** the workflow has no action nodes
- **THEN** the output is exactly `(no action nodes)` and the exit code is 0.

### Requirement: `instance start` creates a fresh run and reports started with optional context

`steer instance start <workflow> <name>` (two positionals) SHALL validate the
workflow then create a fresh instance under `.steer/instances/<name>/` with a
reset context. On success, when no `@context` directive is present, stdout SHALL
be exactly `instance <name>: started`. When the workflow declares a non-empty
`@context`, that text SHALL be appended after a blank line:
`instance <name>: started\n\n<context>`. Instruction text SHALL NOT leak into
start output when no `@context` is present. An invalid or unreadable workflow
exits FAILURE. Evidenced by `run_instance_start`, the
`START_NO_CONTEXT_TEMPLATE`/`START_WITH_CONTEXT_TEMPLATE` constants in
`crates/steer-core/src/template.rs`, and the tests
`instance_start_appends_context_directive`,
`instance_start_without_context_omits_description`.

#### Scenario: start without a context directive

- **WHEN** the workflow has no `@context` directive and the user runs
  `steer instance start <workflow> <name>`
- **THEN** stdout is exactly `instance <name>: started` and the exit code is 0.

#### Scenario: start appends a context directive

- **WHEN** the workflow declares a non-empty `@context` and the user runs
  `steer instance start <workflow> <name>`
- **THEN** stdout is `instance <name>: started`, a blank line, then the context
  text, and the exit code is 0.

#### Scenario: an invalid workflow aborts start

- **WHEN** the workflow is invalid and the user runs
  `steer instance start <workflow> <name>`
- **THEN** the exit code is nonzero and no instance is created.

### Requirement: `instance status` renders running, complete, or halted with optional context

`steer instance status <name>` (one positional) SHALL print
`instance <name>: <status>` where `<status>` is `running`, `complete`, or
`halted: <reason>`. When the workflow declares a non-empty `@context`, that text
SHALL be appended after a blank line. It SHALL exit 0 for a loadable instance.
Evidenced by `run_instance_status`, the
`STATUS_NO_CONTEXT_TEMPLATE`/`STATUS_WITH_CONTEXT_TEMPLATE` constants in
`crates/steer-core/src/template.rs`, and
[instance-lifecycle](openspec/specs/instance-lifecycle/spec.md).

#### Scenario: status strings by state

- **WHEN** the user runs `steer instance status <name>` on an instance in each
  state
- **THEN** a `Running` instance prints `instance <name>: running`, a `Complete`
  instance prints `instance <name>: complete`, and a `Halted(r)` instance prints
  `instance <name>: halted: <r>`.

#### Scenario: status appends a context directive

- **WHEN** the workflow declares a non-empty `@context`
- **THEN** the context text is appended after a blank line on the status output.

### Requirement: `instance step` returns the current instruction without mutating state

`steer instance step <name>` (one positional) SHALL return the rendered
instruction at the current program counter without advancing state. stdout SHALL
be the rendered instruction for a runnable step, exactly `(complete)` when the
run is finished, exactly `(not running)` when the instance is not running, or
`error: <e>` on an error. It SHALL exit 0. Evidenced by `run_instance_step`
mapping `StepOutcome::{Instruction, Complete, NotRunning, Error}` in
`crates/steer-cli/src/main.rs`.

#### Scenario: step returns the current instruction

- **WHEN** the instance is running and the user runs `steer instance step <name>`
- **THEN** stdout is the rendered instruction at the current program counter and
  the exit code is 0.

#### Scenario: step on a finished run

- **WHEN** the instance is `Complete` and the user runs `steer instance step <name>`
- **THEN** stdout is exactly `(complete)` and the exit code is 0.

#### Scenario: step on a not-running instance

- **WHEN** the instance is not running and the user runs
  `steer instance step <name>`
- **THEN** stdout is exactly `(not running)` and the exit code is 0.

### Requirement: `instance check` advances the program counter with fixed outcome tokens

`steer instance check <name>` (one positional) SHALL advance the program counter
and dispatch by node type. stdout SHALL be exactly one of: `advanced`, the
rendered verification instruction, `pending`, `failed`, `(done)`,
`(not running)`, or `error: <e>`. It SHALL exit 0. Per-iteration check state
SHALL NOT leak across loop iterations — each loop iteration MUST require a fresh
report. Evidenced by `run_instance_check` mapping
`CheckOutcome::{Advanced, Instruction, Pending, Failed, Done, NotRunning, Error}`
in `crates/steer-cli/src/main.rs`, the test
`instance_for_loop_check_requires_per_iteration_report`, and
[runtime-check-gate](openspec/specs/runtime-check-gate/spec.md).

#### Scenario: check advances over a non-check node

- **WHEN** the instruction at the program counter is not a verification node
- **THEN** stdout is exactly `advanced` and the exit code is 0.

#### Scenario: check presents a verification node

- **WHEN** the instruction at the program counter is a verification node
- **THEN** stdout is the rendered verification instruction and the exit code is
  0.

#### Scenario: check on a finished run

- **WHEN** the instance has run off the end and the user runs
  `steer instance check <name>`
- **THEN** stdout is exactly `(done)` and the exit code is 0.

#### Scenario: per-iteration check state does not leak

- **WHEN** a `for` loop body contains a check node and the loop runs multiple
  iterations
- **THEN** each iteration MUST require a fresh check report; the previous
  iteration's report MUST NOT satisfy the current iteration's check.

### Requirement: `instance set` writes a typed value and enforces the return type at set time

`steer instance set <name> <var> <value>` (three positionals) SHALL write a
typed value/flag into the instance context. On success stdout SHALL be exactly
`ok`. It SHALL enforce the current op's declared `return` type at set time: if
`<var>` is the assignment target of the op at the current program counter, the
value MUST match the callee's declared return type, and a mismatch SHALL be
rejected before storing. It SHALL exit 0 on success and FAILURE on a type or
store error. Evidenced by `run_instance_set` calling `steer_core::validate_set_value`
before `steer_core::set_value` in `crates/steer-cli/src/main.rs` and the commit
that moved return-type enforcement from check to set (`ea231ae`).

#### Scenario: a correctly typed set succeeds

- **WHEN** the value matches the current op's declared `return` type
- **THEN** stdout is exactly `ok` and the exit code is 0.

#### Scenario: a wrong-typed set is rejected at set time

- **WHEN** `<var>` is the assignment target of the op at the current program
  counter and the value does not match the callee's declared return type
- **THEN** the value is rejected before storing, an error is reported, and the
  exit code is nonzero.

### Requirement: `instance error` halts the run and prints `halted`

`steer instance error <name> <reason>` (two positionals) SHALL report a fatal
failure: it SHALL halt the instance immediately with the given reason verbatim
and stdout SHALL be exactly `halted`. It SHALL exit 0. Evidenced by
`run_instance_error` calling `steer_core::report_error` in
`crates/steer-cli/src/main.rs`.

#### Scenario: error halts the run

- **WHEN** the user runs `steer instance error <name> <reason>`
- **THEN** stdout is exactly `halted`, the instance status becomes
  `Halted(<reason>)`, and the exit code is 0.

### Requirement: Instance names are sanitized before reaching the filesystem

The CLI SHALL reject an unsafe instance name before any filesystem access. A
name equal to `.`, `..`, the empty string, or containing a slash, backslash, or
NUL byte MUST be rejected with a diagnostic of the form "error: invalid instance
name" carrying the offending value, and the process MUST exit FAILURE, preventing
the name from escaping `.steer/instances/`. Evidenced by `instance_dir` in
`crates/steer-cli/src/main.rs`. The full validation contract is normative in
[instance-name-validation](openspec/specs/instance-name-validation/spec.md); the
CLI error string is pinned in
[cli-error-reporting](openspec/specs/cli-error-reporting/spec.md).

#### Scenario: a traversal-style name is rejected

- **WHEN** the user runs an instance command with a name like `../x`
- **THEN** stderr is `error: invalid instance name \`../x\`` and the exit code is
  nonzero, before any instance directory is touched.

#### Scenario: an empty name is rejected

- **WHEN** the user runs an instance command with an empty name
- **THEN** the name is rejected with `error: invalid instance name \`\`` and the
  exit code is nonzero.

### Requirement: The exit-code contract maps success to 0 and every failure to FAILURE

Application logic SHALL emit exactly two exit codes: `ExitCode::SUCCESS` (0) for
every successful command and `ExitCode::FAILURE` (1) for every documented
failure path (unreadable file, parse error, semantic validation failure, bad
instance name, type/store error, IR or context load failure). No other exit
code SHALL originate from application logic. clap's own exit code for unknown
subcommands or argument-parse failures is an inherited part of the external
surface. Evidenced by the uniform `ExitCode::SUCCESS`/`ExitCode::FAILURE`
returns across `run_validate`, `run_simulate`, `run_list`, `run_instance_*`, and
`with_instance_result` in `crates/steer-cli/src/main.rs`.

#### Scenario: a successful command exits 0

- **WHEN** any command completes its documented happy path
- **THEN** the process exits with code 0.

#### Scenario: a failed command exits nonzero

- **WHEN** any command hits a documented failure path
- **THEN** the process exits with code 1.
