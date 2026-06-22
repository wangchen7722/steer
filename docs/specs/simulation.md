# Simulation

> Behavior specs for `steer workflow simulate`: the static dry-run that renders every action node once without executing the workflow.

## Scenario: render all action nodes in order
- **WHEN** the user runs `steer workflow simulate <wf>`
- **THEN** the CLI prints each action node's rendered instruction, numbered, in
  source order.

## Scenario: static walk
- **WHEN** the workflow has loops and branches
- **THEN** each action node is shown once; loops are not expanded and both
  branches of an `if` are shown.

## Scenario: user-function call sites are not action instructions
- **WHEN** a workflow calls a user function
- **THEN** the call site itself produces no instruction; action nodes in the
  function body are shown during the static walk.

## Scenario: nested action calls are rendered
- **WHEN** an action call appears inside an expression that simulation visits
- **THEN** simulation renders that action node and threads assignment targets
  into `render_call` when available.

## Scenario: empty workflow
- **WHEN** the workflow has no action nodes
- **THEN** simulate prints `(no action nodes)`.
