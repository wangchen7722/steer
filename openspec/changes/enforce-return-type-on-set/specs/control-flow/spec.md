## MODIFIED Requirements

### Requirement: judge and check are distinct mechanisms

When the author needs a judgment result in a condition, `judge("...")` returns
a boolean into a variable. The VM enforces that boolean type at `check` time:
if the agent sets a non-boolean value (e.g. a JSON object) into the variable,
`check` returns `Failed` with a reason and the op does not advance, so a
truthy non-bool value cannot fool a downstream `until`/`if` condition. When the
author needs verify-and-retry behavior, `task("...", check="...")` uses the
runtime checked flow (unaffected by return-type enforcement).

#### Scenario: judge accepts a boolean

- **WHEN** the author writes `covered = judge("...")` and the agent sets
  `covered` to `true` or `false`
- **THEN** `check` advances and `until covered` / `if not covered` evaluate
  the boolean.

#### Scenario: judge rejects a non-boolean so conditions cannot be fooled

- **WHEN** the agent sets `covered` to a JSON object instead of a boolean
- **THEN** `check` returns `Failed` with a reason, the op does not advance,
  and `until covered` is not evaluated against the wrong-typed value.
