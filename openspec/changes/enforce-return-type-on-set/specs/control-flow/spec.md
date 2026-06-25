## MODIFIED Requirements

### Requirement: judge and check are distinct mechanisms

When the author needs a judgment result in a condition, `judge("...")` returns
a boolean into a variable. The engine enforces that boolean type at `set`
time: if the agent sets a non-boolean value (e.g. a JSON object) into the
variable, `set` is rejected with a reason and the value is not stored, so a
truthy non-bool value can never reach a downstream `until`/`if` condition.
When the author needs verify-and-retry behavior, `task("...", check="...")`
uses the runtime checked flow (unaffected by return-type enforcement).

#### Scenario: judge accepts a boolean

- **WHEN** the author writes `covered = judge("...")` and the agent sets
  `covered` to `true` or `false`
- **THEN** `set` succeeds and `until covered` / `if not covered` evaluate the
  boolean.

#### Scenario: judge rejects a non-boolean so conditions cannot be fooled

- **WHEN** the agent sets `covered` to a JSON object instead of a boolean
- **THEN** `set` is rejected with a reason, `covered` is not stored, and
  `until covered` is never evaluated against a wrong-typed value.
