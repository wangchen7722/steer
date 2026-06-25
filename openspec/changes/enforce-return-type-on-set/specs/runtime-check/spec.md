## ADDED Requirements

### Requirement: Value-op check enforces the callee's declared return type

When `check` runs on an assigned value op (`x = callee(...)`, no `check=`
clause), the VM SHALL resolve the callee's template and compare the value the
agent set against the callee's declared `return` `ParamKind`. A value whose
`Value` variant does not match the declared kind MUST be rejected: `check`
returns `Failed`, stores a failure reason on the current step, and increments
the retry count, so the next `step` asks the agent to re-issue the correct
`steer instance set` command. The `checked` (`check=`) path is unaffected and
keeps accepting `true`/`false` and `{"passed":bool,"reason":"..."}`.

#### Scenario: bool return accepts a boolean value

- **WHEN** `covered = judge("...")` runs and the agent sets `covered` to
  `true` (or `false`)
- **THEN** `check` advances past the op (`Advanced`).

#### Scenario: bool return rejects a non-boolean value

- **WHEN** `covered = judge("...")` runs and the agent sets `covered` to a
  JSON object such as `{"verdict":"COVERED",...}`
- **THEN** `check` returns `Failed` with a reason stating a boolean was
  expected, stores the reason on the step, and the op does not advance.

#### Scenario: string return accepts a string value

- **WHEN** `x = task("...", return="...")` runs and the agent sets `x` to a
  bare string
- **THEN** `check` advances past the op (`Advanced`).

#### Scenario: string return rejects an object value

- **WHEN** `x = task("...", return="...")` runs and the agent sets `x` to a
  JSON object
- **THEN** `check` returns `Failed` with a reason stating a string was
  expected and that structured data is not supported by `return:string`.

#### Scenario: retry reason drives a corrective re-set

- **WHEN** a value op's previous `check` failed with a return-type reason
- **THEN** the next `step` for the same op appends that reason and asks the
  agent to set the correct type before checking again.

#### Scenario: undeclared return type is not enforced

- **WHEN** a value op's callee has `return: none`, no `return` spec, or is a
  bare (unassigned) call
- **THEN** `check` does not type-check the value and keeps the existing
  key-presence / immediate-advance behavior.

## MODIFIED Requirements

### Requirement: value-op check waits for the value

When `check` runs on an assigned value op before its value is set, it returns
`Pending`. Once `set` supplies a value whose type matches the callee's
declared `return` `ParamKind`, `check` advances. If the supplied value's type
does not match, `check` returns `Failed` with a reason (see *Value-op check
enforces the callee's declared return type*) instead of advancing.

#### Scenario: pending until a correctly-typed value is set

- **WHEN** `check` runs on an assigned value op before its value is set
- **THEN** it returns `Pending`; once `set` supplies a value whose type
  matches the declared `return` kind, `check` advances.

#### Scenario: wrong-typed value does not advance

- **WHEN** `set` supplies a value whose type does not match the declared
  `return` kind
- **THEN** `check` returns `Failed` with a reason and does not advance.
