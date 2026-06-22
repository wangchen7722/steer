# Instance Lifecycle

> Behavior specs for instance management: `start` semantics, instance-name validation, typed `set`, fatal `error`, and resume across CLI calls.

## Scenario: start creates a fresh instance
- **WHEN** the user runs `steer instance start <wf> <name>`
- **THEN** a fresh instance is created under `.steer/instances/<name>/`,
  replacing any previous instance with the same valid name.

## Scenario: invalid instance names are rejected
- **WHEN** the instance name is empty, `.`, `..`, or contains `/`, `\`, or a
  NUL byte
- **THEN** the CLI rejects it before touching `.steer/instances`.

## Scenario: set writes typed values
- **WHEN** the user runs `steer instance set <name> <var> <value>`
- **THEN** JSON literals are parsed as typed values, and bare strings remain
  strings.

## Scenario: error halts and status reports state
- **WHEN** the user runs `steer instance error <name> <reason>` then `status`
- **THEN** the run is `Halted` and `status` reports it.

## Scenario: resume across CLI calls
- **WHEN** `step`, `check`, and `set` are issued as separate CLI invocations
- **THEN** persisted context lets the run continue from the same PC and state.
