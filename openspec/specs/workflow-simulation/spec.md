# Workflow Simulation

## Purpose

`steer workflow simulate <workflow>` projects the instruction trace a workflow
would drive, by walking the parsed module in source order and rendering every
action-node call once. It is a static, non-executing projection: it resolves no
variables, inlines no user functions, expands no loops, invokes no agent, writes
no instance state, and produces no check pass/fail decisions. Its purpose is to
let an author preview the agent-facing instruction sequence (and validate the
workflow's structure) without running it.

## Requirements

### Requirement: Simulation renders action nodes in source order

The simulation SHALL walk the parsed workflow module in source order and emit
one rendered instruction per action-node call, preserving the order in which the
calls appear in the source. Evidenced by `simulate()` / `walk_block` /
`walk_stmt` in `crates/steer-core/src/simulate.rs` and the test
`renders_in_source_order`.

#### Scenario: a sequence of action nodes is rendered in order

- **WHEN** a workflow contains several action-node calls in sequence
- **THEN** simulation emits one rendered instruction per call, in source order.

#### Scenario: the CLI numbers and separates steps

- **WHEN** the user runs `steer workflow simulate <workflow>` on a workflow with
  action nodes
- **THEN** each step is printed as a `[<i>] <callee>` header followed by the
  rendered instruction body and a blank-line separator.

### Requirement: User-function call sites are skipped but their bodies are rendered

The simulation SHALL treat a call whose callee is a user-defined function as a
non-action call site (it is skipped, not emitted), while action nodes nested
inside that function's body SHALL still be rendered during the walk. Evidenced
by `render_call_node` guarding on `funcs.contains(&call.callee)` and the test
`skips_user_function_calls_but_renders_their_bodies`.

#### Scenario: a user-function call site is not emitted as a step

- **WHEN** a workflow defines a function and calls it
- **THEN** the call site itself produces no simulated step.

#### Scenario: action nodes inside a function body are rendered

- **WHEN** a user function body contains action-node calls
- **THEN** those action nodes are rendered as steps when the function's body is
  walked.

### Requirement: Loops and conditionals are projected structurally, not executed

The simulation SHALL render a `loop ... until` body and a `for ... in` body
exactly once regardless of exit condition or iterable, and SHALL render every
`if`/`elseif`/`else` branch regardless of condition value, because the walk
never evaluates conditions or iterates. Runtime `{var}` interpolations SHALL be
preserved as literal placeholders rather than resolved. Evidenced by `walk_stmt`
for `LoopUntil`/`For`/`If` and the tests `loop_body_shown_once`,
`renders_both_if_branches`.

#### Scenario: a loop body appears once

- **WHEN** a workflow contains a `loop ... until` or `for ... in` with action
  nodes in its body
- **THEN** the body's action nodes appear exactly once in the projection,
  independent of the exit condition or iterable.

#### Scenario: both branches of an if are shown

- **WHEN** a workflow contains an `if`/`elseif`/`else` with action nodes in
  multiple branches
- **THEN** every branch's action nodes are rendered, regardless of the branch
  conditions.

### Requirement: An action call bound to a target renders a set prompt

The simulation SHALL thread the assignment-target name (or `return` slot) into
the rendered instruction when an action call is the right-hand side of an
assignment or a `return` value, so the rendered instruction tells the agent
which variable to set. A bare action call (no receiver) SHALL NOT produce a
`steer instance set` prompt. Evidenced by `render_call_node` /
`assignment_call_simulates_set_prompt_with_target`.

#### Scenario: an assigned action call shows the target

- **WHEN** an action call is the right-hand side of an assignment such as
  `x = ask(...)`
- **THEN** the rendered instruction names `x` as the variable the agent should
  set.

#### Scenario: a bare action call does not prompt a set

- **WHEN** an action call is used as a bare statement with no receiver
- **THEN** the rendered instruction does not direct the agent to run
  `steer instance set`.

### Requirement: An empty workflow reports no action nodes

The simulation SHALL produce no steps for an empty or comment-only workflow, and
the CLI SHALL print exactly `(no action nodes)` and exit successfully.
Simulation SHALL be infallible: it takes a pre-parsed module and returns a vector
of steps without a `Result`, and parse/load failures are handled upstream before
simulation runs. Evidenced by `empty_workflow_yields_nothing` and the
`run_simulate` exit code in `crates/steer-cli/src/main.rs`.

#### Scenario: empty workflow output

- **WHEN** the user runs `steer workflow simulate` on an empty or comment-only
  workflow
- **THEN** the output is exactly `(no action nodes)` and the exit code is 0.

### Requirement: Action calls nested in compound expressions are rendered

The simulation SHALL render action calls nested anywhere inside a compound
expression — as a binary or unary operand, a list-literal element, or a call
argument — by recursively descending into sub-expressions, in addition to
rendering calls in statement, assignment-right-hand-side, return, condition, and
iterable positions. Evidenced by `render_if_call` recursing through `Binary`,
`Unary`, `List`, and each call argument in `crates/steer-core/src/simulate.rs`.

#### Scenario: an action call inside a binary expression is rendered

- **WHEN** a workflow contains an action call nested as an operand of a binary
  expression
- **THEN** the nested call is rendered as a step during simulation.
