# CLI Surface

> Behavior specs for the `steer` command-line interface: version output and subcommand structure.

## Scenario: print version
- **WHEN** the user runs `steer --version`
- **THEN** the binary prints a version string and exits successfully.

## Scenario: subcommands are recognized
- **WHEN** the user runs `steer workflow {validate,simulate,list}` or
  `steer instance {start,status,step,check,set,error}`
- **THEN** the CLI parses the subcommand and its positional arguments.
