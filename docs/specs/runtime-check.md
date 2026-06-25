# Runtime Check Flow

> Behavior specs for the `step` / `check` cycle: pausing at action nodes, rendering verification instructions, the `checked` flag, retry with failure reason, and the pending/advanced/failed outcomes.

## Scenario: step pauses at the next action node
- **WHEN** `step` is called on a running context
- **THEN** it executes control instructions and pauses at the next action node,
  returning the rendered task instruction.

## Scenario: check renders verification instruction
- **WHEN** `check` runs on an action node with `check="run tests"`
- **THEN** it returns an instruction containing `run tests` and the runtime
  appended reporting commands:
  `steer instance set <name> checked {"passed":true}` and
  `steer instance set <name> checked {"passed":false,"reason":"<why it failed>"}`.

## Scenario: failed check requires a reason
- **WHEN** the user runs `steer instance set <name> checked false`
- **THEN** the command is rejected.
- **WHEN** the user runs
  `steer instance set <name> checked '{"passed":false,"reason":"tests failed"}'`
- **THEN** `check` returns `failed` and stores the reason on the current step.

## Scenario: retry instruction includes the previous failure reason
- **WHEN** a step's previous verification failed with a reason
- **THEN** the next `step` for the same action appends that reason and asks the
  agent to address it before checking again.

## Scenario: passing check advances
- **WHEN** the user runs `steer instance set <name> checked true` or sets
  `{"passed":true}`
- **THEN** `check` advances past the current action node.

## Scenario: value-op check waits for the value
- **WHEN** `check` runs on an assigned value op before its value is set
- **THEN** it returns `Pending`; once `set` supplies a value, `check` advances.
  `check` performs no return-type validation for value ops — that happens at
  `set` time (see the next scenario), because a value op has no `check=` gate
  and the agent could otherwise `set` and `step` past the op without checking.

## Scenario: set enforces the current op's declared return type
- **WHEN** the agent runs `steer instance set <name> <var> <value>` and `<var>`
  is the assignment target of the value op at the current program counter
- **AND** the op's callee declares a `return` type (`bool` or `string`)
- **AND** the value's type does not match (e.g. a JSON object, or `false`, set
  into a `bool` variable such as `covered = judge(...)`; or `false`/an object
  set into a `string` variable such as `bug_slug = ask(..., return=...)`)
- **THEN** `set` is rejected with a reason naming the expected type, exits
  non-zero, and does NOT store the value. A `bool` return accepts only
  `true`/`false`; a `string` return accepts only a string and rejects any other
  type with a reason stating structured data is not supported by
  `return:string`.

## Scenario: set of a non-target variable is not type-checked
- **WHEN** the agent sets a variable that is not the current op's assignment
  target (e.g. a workflow local, or an already-completed op's variable)
- **THEN** `set` is not constrained by any callee return type and accepts the
  value.

## Scenario: undeclared return types are not type-checked at set
- **WHEN** the current op's callee declares `return: none`, has no `return`
  spec, or is a bare (unassigned) call
- **THEN** `set` does not enforce a return type and accepts the value.

## Scenario: bare op check advances immediately
- **WHEN** `check` runs on an action node with no value target and no `check`
  clause
- **THEN** it advances immediately.
