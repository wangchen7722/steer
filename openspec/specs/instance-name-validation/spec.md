# Instance Name Validation

## Purpose

Gates every instance subcommand by rejecting names that could escape the
`.steer/instances/` directory, confining each instance path to a single
verbatim path segment before any filesystem access occurs.

## Requirements

### Requirement: Invalid names are rejected before any filesystem access

`steer instance <action> <name>` SHALL reject a name and exit FAILURE without
creating, modifying, or reading any directory when the name is empty, equals
`.` or `..`, or contains any of `/`, `\`, or a NUL byte (`\0`). On rejection
the CLI SHALL print exactly `error: invalid instance name \`<name>\``.
Evidenced by `instance_dir` in `crates/steer-cli/src/main.rs`.

#### Scenario: empty, dot, and dot-dot are rejected

- **WHEN** the user runs `steer instance start ""`, `steer instance start .`,
  or `steer instance start ..`
- **THEN** the CLI prints `error: invalid instance name \`<name>\`` and exits
  FAILURE without creating any directory.

#### Scenario: path separators and NUL are rejected

- **WHEN** the user runs `steer instance start` with a name containing `/`,
  `\`, or a NUL byte
- **THEN** the CLI prints `error: invalid instance name \`<name>\`` and exits
  FAILURE without touching the filesystem.

### Requirement: Valid names are joined verbatim as the path segment

A name that passes validation SHALL be used verbatim as the final path segment
under `.steer/instances/` with no normalization. Names containing spaces,
dots elsewhere, or unicode SHALL be accepted as-is. Evidenced by
`instance_dir` in `crates/steer-cli/src/main.rs` (whose comment states the
path-escape rationale) and `docs/specs/instance.md`.

#### Scenario: spaces and unicode are accepted verbatim

- **WHEN** the user starts an instance named `my run` or `测试`
- **THEN** the instance directory is `.steer/instances/my run` or
  `.steer/instances/测试` respectively, with no escaping or normalization.

### Requirement: Every instance subcommand applies the same validation

The name validation SHALL be applied identically by every instance subcommand
(start, step, check, set, error, status), so no subcommand can reach the
filesystem with an escaping name. Evidenced by `instance_dir` being shared
across `run_instance_start`, `with_instance_result`, and
`run_instance_status` in `crates/steer-cli/src/main.rs`. This guard protects
the directory layout pinned by
[instance-persistence](openspec/specs/instance-persistence/spec.md).

#### Scenario: status also validates

- **WHEN** the user runs `steer instance status ..`
- **THEN** the CLI prints `error: invalid instance name \`..\`` and exits
  FAILURE without reading the filesystem.

#### Scenario: a valid name on any subcommand proceeds

- **WHEN** the user runs any instance subcommand with a valid name
- **THEN** the command proceeds past name validation to its normal behavior.
