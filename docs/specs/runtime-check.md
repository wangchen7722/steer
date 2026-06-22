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
- **THEN** it returns `Pending`; once `set` supplies the value, `check`
  advances.

## Scenario: bare op check advances immediately
- **WHEN** `check` runs on an action node with no value target and no `check`
  clause
- **THEN** it advances immediately.
