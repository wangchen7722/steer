# Loop And Branch Conditions

> Behavior specs for control-flow conditions: the steer-side predicate purity, PC/ALU division of labor, post-test loop semantics, and the `judge` vs `check` distinction.

## Scenario: conditions are steer-side predicates
- **WHEN** an `if cond` or `until cond` is evaluated
- **THEN** `cond` is a pure expression over context variables, not an agent op.

## Scenario: world-dependent exit conditions live in the loop body
- **WHEN** a loop should exit based on outside-world state
- **THEN** the body runs an action node that sets a context variable, and the
  `until` or `if` condition reads that variable.

## Scenario: loop-until is post-test
- **WHEN** a `loop ... until cond` runs
- **THEN** the body runs at least once, then `cond` is tested.

## Scenario: judge and check are distinct mechanisms
- **WHEN** the author needs a judgment result in a condition
- **THEN** `judge("...")` returns a boolean into a variable, and `set`
  enforces that boolean type: a non-boolean value (e.g. a JSON object) set by
  the agent is rejected at `set` with a reason and not stored, so a downstream
  `until`/`if` condition cannot be fooled by a truthy non-bool value.
- **WHEN** the author needs verify-and-retry behavior
- **THEN** `task("...", check="...")` uses the runtime checked flow.
