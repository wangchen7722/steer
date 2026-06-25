## ADDED Requirements

### Requirement: set enforces the current op's declared return type

When the agent runs `steer instance set <name> <var> <value>` and `<var>` is
the assignment target (`into`) of the value op at the current program counter,
the engine SHALL resolve the callee's template and compare the value against
the callee's declared `return` `ParamKind`. A value whose `Value` variant does
not match the declared kind MUST be rejected: `set` exits non-zero with a
reason, and the value is **not stored**. The `checked` (`check=`) path is
unaffected and keeps accepting `true`/`false` and
`{"passed":bool,"reason":"..."}`.

#### Scenario: bool return accepts a boolean value

- **WHEN** `covered = judge("...")` is the current op and the agent sets
  `covered` to `true` (or `false`)
- **THEN** `set` succeeds and stores the boolean.

#### Scenario: bool return rejects a non-boolean value

- **WHEN** `covered = judge("...")` is the current op and the agent sets
  `covered` to a JSON object such as `{"verdict":"COVERED",...}`
- **THEN** `set` is rejected with a reason stating a boolean was expected, and
  `covered` is not stored.

#### Scenario: string return accepts a string value

- **WHEN** `x = ask("...", return="...")` is the current op and the agent sets
  `x` to a bare string
- **THEN** `set` succeeds and stores the string.

#### Scenario: string return rejects a non-string value

- **WHEN** `x = ask("...", return="...")` is the current op and the agent sets
  `x` to a non-string value (a boolean such as `false`, or a JSON object)
- **THEN** `set` is rejected with a reason stating a string was expected and
  that structured data is not supported by `return:string`, and `x` is not
  stored.

#### Scenario: set of a non-target variable is not type-checked

- **WHEN** the agent sets a variable that is not the current op's assignment
  target (e.g. a workflow local, or an already-completed op's variable)
- **THEN** `set` is not constrained by any callee return type and accepts the
  value.

#### Scenario: undeclared return type is not enforced

- **WHEN** the current op's callee has `return: none`, no `return` spec, or is
  a bare (unassigned) call
- **THEN** `set` does not type-check the value and accepts it.

## MODIFIED Requirements

### Requirement: value-op check waits for the value

When `check` runs on an assigned value op before its value is set, it returns
`Pending`. Once `set` supplies a value, `check` advances. `check` performs no
return-type validation for value ops — type enforcement happens at `set` time
(see *set enforces the current op's declared return type*), because a value op
has no `check=` gate and the agent could otherwise `set` and `step` past the
op without calling `check`.

#### Scenario: pending until a value is set

- **WHEN** `check` runs on an assigned value op before its value is set
- **THEN** it returns `Pending`; once `set` supplies a value, `check` advances.
