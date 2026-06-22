# Validation

> Behavior specs for `steer workflow validate`: semantic checks on assignments, argument types, reserved names, function placement, and error reporting.

## Scenario: valid workflow
- **WHEN** the user runs `steer workflow validate <valid-file>`
- **THEN** the CLI prints `<path>: OK` and exits 0.

## Scenario: value node assigned without `return`
- **WHEN** a value node (`task`, `ask`, `command`, or `collect`) is assigned
  without `return=`
- **THEN** validation reports an error.

## Scenario: bare task without `return` is allowed
- **WHEN** `task(...)` is used only for side effects
- **THEN** validation reports no error.

## Scenario: `return` prompt is required only for a real assignment target
- **WHEN** a bare call includes `return=`
- **THEN** the rendered instruction does not ask the agent to run
  `steer instance set <name> <var>` because no variable is receiving the value.

## Scenario: argument type rules
- **WHEN** `produce=` is not a list literal, `check=` or `return=` is not a
  string literal, or a function/parameter/named argument is duplicated
- **THEN** validation reports an error.

## Scenario: reserved runtime names
- **WHEN** a workflow assigns to `checked` or to a name beginning with `__`
- **THEN** validation rejects the workflow.

## Scenario: functions are top-level only
- **WHEN** a `func` appears inside another statement body
- **THEN** validation rejects it.

## Scenario: parse error reports a location
- **WHEN** the workflow has a syntax error
- **THEN** the CLI prints the message with `at line L, col C` and exits
  non-zero.

## Scenario: shipped workflows validate and simulate
- **WHEN** each workflow under `.steer/workflows/` is validated and simulated
- **THEN** all workflows pass validation and render without errors (the
  repository ships `openspec-propose.steer` and `openspec-apply.steer`).
