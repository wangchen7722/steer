# Workflow File Discovery

> Behavior specs for resolving the `<workflow>` argument: explicit path precedence, flat lookup under `.steer/workflows/`, and `.steer` extension auto-append.

## Scenario: workflow path is resolved under `.steer/workflows/`
- **WHEN** the user runs `instance start`, `workflow validate`, or
  `workflow simulate` with a path that is not an existing file in the current
  directory
- **THEN** the CLI falls back to a flat lookup under `.steer/workflows/` by file
  name, so `bugfix-loop.steer` resolves to `.steer/workflows/bugfix-loop.steer`.

## Scenario: a bare name auto-appends `.steer`
- **WHEN** the user passes a name with no extension, e.g. `bugfix-loop`
- **THEN** discovery tries `.steer/workflows/bugfix-loop.steer`.

## Scenario: an explicit path takes precedence
- **WHEN** a file matching the given path exists in the current directory
- **THEN** it is read directly and the `.steer/workflows/` fallback is not
  consulted, even if a same-named file exists there.

## Scenario: nothing matches keeps the original error
- **WHEN** the path is not found at the given location and no same-named file
  exists under `.steer/workflows/`
- **THEN** the CLI reports `cannot read <original path>` as before.
