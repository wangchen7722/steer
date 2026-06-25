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
- **THEN** it returns `Pending`; once `set` supplies a value whose type matches
  the callee's declared `return` type, `check` advances.

## Scenario: value-op check enforces the declared return type
- **WHEN** `check` runs on an assigned value op whose callee declares a
  `return` type (`bool` or `string`)
- **AND** the agent has set a value whose `Value` variant does not match that
  type (e.g. a JSON object set into a `bool` variable such as
  `covered = judge(...)`)
- **THEN** `check` returns `Failed` with a reason naming the expected type,
  stores the reason on the step, increments the retry count, and does NOT
  advance the program counter. A `bool` return accepts only `true`/`false`; a
  `string` return accepts only a string and rejects an object with a reason
  stating structured data is not supported by `return:string`.

## Scenario: type failure surfaces as a retry reason on the next step
- **WHEN** a value op's previous `check` failed with a return-type reason
- **THEN** the next `step` for the same op appends that reason as a retry
  context, asking the agent to re-issue the correct `steer instance set`
  command before checking again.

## Scenario: undeclared return types are not type-checked
- **WHEN** a value op's callee declares `return: none`, has no `return` spec,
  or is a bare (unassigned) call
- **THEN** `check` does not enforce a return type and keeps the existing
  key-presence / immediate-advance behavior.

## Scenario: bare op check advances immediately
- **WHEN** `check` runs on an action node with no value target and no `check`
  clause
- **THEN** it advances immediately.
