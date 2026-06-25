# Instance Persistence

## Purpose

Defines the on-disk layout of a steer instance under
`.steer/instances/<name>/` and pins the exact serialized `context.json`
schema so external tooling can read instance state as a compatibility
surface, while the CLI resumes runs transparently via atomic staging.

## Requirements

### Requirement: An instance is a self-contained directory of two files

An instance SHALL be a directory `.steer/instances/<name>/` containing exactly
two files: `workflow.steer` (a byte-for-byte copy of the workflow source
supplied at start) and `context.json` (the serialized `Context`). The IR
SHALL NOT be persisted; it is re-parsed and re-lowered from `workflow.steer`
on every command, so a run SHALL NOT depend on the original
`.steer/workflows/<name>.steer` continuing to exist or staying unchanged
after start. Evidenced by the `WORKFLOW_FILE`/`CONTEXT_FILE` constants and
`start_instance` in `crates/steer-core/src/storage.rs`, and the test
`start_creates_fresh_context_and_workflow`.

#### Scenario: start lays out both files

- **WHEN** `start_instance` runs for a workflow
- **THEN** the directory `.steer/instances/<name>/` contains `workflow.steer`
  (byte-identical to the source) and `context.json`.

#### Scenario: a run is independent of the original workflow file

- **WHEN** the original `.steer/workflows/<name>.steer` is removed or altered
  after start
- **THEN** subsequent instance commands still succeed because the source
  snapshot lives inside the instance directory.

### Requirement: context.json pins the top-level field shape

`context.json` SHALL be pretty-printed JSON with exactly the top-level keys
`pc` (integer), `status` (per the Status enum), `vars` (object of name to
`Value`), `frames` (array, empty at rest), `steps` (object, see below), and
`meta` (object). The `meta` key SHALL be `#[serde(default)]` so older files
without it still load. Evidenced by `struct Context` and the live
`.steer/instances/*/context.json` files (e.g. `bugfix-loop`, `gen-specs`,
`workflow-marketplace`).

#### Scenario: round-trip preserves all fields

- **WHEN** a context is saved and reloaded
- **THEN** `load_context` returns a context equal in `pc`/`status`/`vars`/
  `frames`/`steps`/`meta` to the original.

#### Scenario: a file lacking meta still loads

- **WHEN** `load_context` reads an older `context.json` with no `meta` key
- **THEN** it loads successfully with `meta` at its default.

### Requirement: pc is a JSON integer and status is an externally-tagged enum

The `pc` field SHALL be a JSON integer (u32). The `status` field SHALL be the
externally-tagged serde rendering of the three-valued Status enum: the string
`"Running"`, the string `"Complete"`, or the single-field object
`{"Halted": "<reason>"}`. Evidenced by `enum Status` in
`crates/steer-core/src/context.rs` and the status values observed in live
`context.json` files.

#### Scenario: running and complete render as bare strings

- **WHEN** a running instance is saved
- **THEN** `status` serializes to `"Running"`.
- **WHEN** a complete instance is saved
- **THEN** `status` serializes to `"Complete"`.

#### Scenario: halted renders as a tagged object

- **WHEN** a halted instance is saved with reason `<r>`
- **THEN** `status` serializes to `{"Halted": "<r>"}`.

### Requirement: steps is a string-keyed object keyed by program counter

The `steps` field SHALL be a JSON object whose keys are the program-counter
index serialized as decimal strings (e.g. `"1"`, `"6"`), because the in-memory
`HashMap<u32, StepState>` is string-keyed in JSON. Each value SHALL be an
object with fields `checked`, `failure_reason`, and `retry_count`. The
`checked` field SHALL be the `#[serde(untagged)]` rendering of `CheckedReport`
(`null`, `true`, `false`, or `{"passed": bool, "reason": string|null}`). The
`failure_reason` field SHALL be a string or null. The `retry_count` field
SHALL be a number (u32) and SHALL be `#[serde(default)]` so absent maps to 0.
Evidenced by `StepState`/`CheckedReport` in `crates/steer-core/src/context.rs`
and the live `steps` blocks in `.steer/instances/*/context.json`.

#### Scenario: steps are keyed by decimal pc strings

- **WHEN** a context with checked steps at pc 1 and 6 is saved
- **THEN** the `steps` object has keys `"1"` and `"6"`.

#### Scenario: a file lacking retry_count still loads as zero

- **WHEN** `load_context` reads an older `context.json` whose `StepState`
  entries omit `retry_count`
- **THEN** the entries load with `retry_count == 0`.

### Requirement: vars values are externally-tagged Value variants

Each entry in the `vars` object SHALL be a `Value` rendered as an
externally-tagged enum: `{"Str": "<string>"}`, `{"Int": <number>}`,
`{"Bool": true|false}`, `{"List": [<Value>, ...]}`, `Null` as JSON `null`,
`Float` as a JSON number, and `Object` as a JSON object. Evidenced by
`enum Value` in `crates/steer-core/src/value.rs` and the live `vars` blocks.

#### Scenario: string and integer values tag their variant

- **WHEN** a context with a string var `s` and an integer var `n` is saved
- **THEN** `vars.s` serializes to `{"Str": "<...>"}` and `vars.n` to
  `{"Int": <...>}`.

#### Scenario: null renders as bare null

- **WHEN** a context with a `Null`-typed variable is saved
- **THEN** its `vars` entry serializes to JSON `null`.

### Requirement: meta records template_dir and context directives

The `meta` object SHALL have fields `template_dir` (string or null) and
`context` (string or null), each `#[serde(default)]`. Evidenced by
`struct WorkflowMeta` in `crates/steer-core/src/context.rs` and live `meta`
blocks. The directives feeding `meta` are extracted by
[workflow-directive-extraction](openspec/specs/workflow-directive-extraction/spec.md).

#### Scenario: meta persists a template and context

- **WHEN** a workflow declares `@template` and `@context` directives and an
  instance is started
- **THEN** `context.json` records the rendered values in `meta.template_dir`
  and `meta.context`.

#### Scenario: a meta subfield absent still loads

- **WHEN** `load_context` reads a `meta` object missing one subfield
- **THEN** it loads with that subfield at its default.

### Requirement: start stages atomically and replaces any prior instance

`start_instance` SHALL stage the new instance into a sibling
`<dir>.steer-new` temporary directory, write both files there, then remove any
pre-existing instance directory and rename the staging directory into place,
so a crash mid-write leaves either the previous instance intact or none —
never a half-written instance. Re-running `start` on an existing name SHALL
fully replace the directory and reset `pc` to 0. Evidenced by `start_instance`
in `crates/steer-core/src/storage.rs` and the test `start_clears_existing_instance`.

#### Scenario: start is crash-safe

- **WHEN** `start_instance` is interrupted while writing
- **THEN** the on-disk state is either the previous instance or absent, never
  a partially written instance directory.

#### Scenario: re-start resets the run

- **WHEN** `start` is re-run on an existing instance name
- **THEN** the directory is replaced and the context is reset to `pc == 0`,
  `status == Running`.

### Requirement: Storage failures surface as stable InstanceError messages

Missing or corrupt instance files SHALL surface as `InstanceError` variants
with stable messages: `InstanceError::Io` (with the raw `io::Error` Display)
when `workflow.steer` is missing or unreadable, `InstanceError::Parse`
(`"workflow did not parse: <parse>"`) when the snapshot no longer parses, and
`InstanceError::Json` (`"invalid context.json: <serde>"`) when `context.json`
is malformed. The CLI SHALL print `error: <message>` and exit FAILURE.
Evidenced by `load_ir`/`load_context`/`save_context` and the
`with_instance_result` error path in `crates/steer-cli/src/main.rs`.

#### Scenario: a missing workflow snapshot fails with Io

- **WHEN** an instance directory has no `workflow.steer`
- **THEN** the command fails with `InstanceError::Io` and the CLI prints
  `error: <io message>` and exits FAILURE.

#### Scenario: a malformed context.json fails with Json

- **WHEN** `context.json` is not valid JSON for the schema
- **THEN** the command fails with `InstanceError::Json` whose message is
  `invalid context.json: <serde>` and the CLI exits FAILURE.

### Requirement: Each frames entry pins the call-stack shape

A non-empty `frames` array SHALL consist of one object per in-flight
user-function call, each with the fields `return_pc` (integer, the PC to resume
at), `into` (string or null, the assignment target for the return value), and
`saved_vars` (object of the caller's variables saved across the call). This is
the persisted call-stack shape external tooling could observe in a context paused
mid-call. Evidenced by `struct Frame` in `crates/steer-core/src/context.rs`.

#### Scenario: a frame entry carries its three fields

- **WHEN** a context paused inside a user-function call is saved
- **THEN** each `frames` entry has `return_pc`, `into`, and `saved_vars`.

#### Scenario: frames is empty at rest

- **WHEN** a context is saved between top-level steps with no call in flight
- **THEN** the `frames` array is empty.
