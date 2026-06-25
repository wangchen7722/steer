# Template Context Binding

## Purpose

Define how steer binds a call's arguments and runtime metadata to the Jinja2 render context
that produces agent-facing instruction text: the always-present steer_target and
steer_instance variables, the first-positional instruction binding, the suppression of check
and bare-call return, the scope-or-literal evaluation, and the preservation of single-brace
placeholders.

## Requirements

### Requirement: The context always carries steer_target and steer_instance

The render context SHALL always include a steer_target entry set to the assignment-receiving
variable name, or to the placeholder token when the call is bare, and a steer_instance entry
set to the running instance name. Templates rely on these two variables to emit the
`steer instance set` prompt that tells the agent where to report a value. Evidenced by
`build_context` in `crates/steer-core/src/template.rs`.

#### Scenario: a bare call targets the placeholder

- **WHEN** a call is used as a bare statement with no receiver
- **THEN** steer_target is the placeholder token and the instance name fills steer_instance.

#### Scenario: an assigned call targets the variable

- **WHEN** a call is the right-hand side of an assignment to a variable
- **THEN** steer_target is that variable's name.

### Requirement: The first positional argument binds as instruction

The renderer SHALL bind the first positional argument of a call to the context under the name
instruction, and SHALL bind each remaining named argument under its own name, except where a
named argument is explicitly suppressed. Evidenced by `build_context` in
`crates/steer-core/src/template.rs` and the test `render_task_with_all_params`.

#### Scenario: the first positional becomes instruction

- **WHEN** a call provides a first positional argument
- **THEN** that argument's value is bound as instruction in the render context.

### Requirement: check is never exposed to the instruction template

The renderer SHALL NOT bind the check argument to the instruction render context. The check
value is consumed only by the VM through the on_check rendering path and SHALL never appear as
a variable available to a node body template. Evidenced by `build_context` skipping `check` in
`crates/steer-core/src/template.rs`, and the test `render_task_with_all_params` asserting the
check value does not leak into the rendered instruction.

#### Scenario: a check argument does not reach the body

- **WHEN** a call provides a check argument
- **THEN** the rendered instruction body contains no variable binding for check.

### Requirement: return is suppressed on bare calls but exposed otherwise

The renderer SHALL suppress the return argument on a bare call (one with no receiver), so a
bare value call does not emit a set prompt, and SHALL expose return to the template when the
call has a known receiver (an assignment or a return value). Exposing return is what lets a
template's conditional and interpolation produce a set prompt in the rendered instruction.
Evidenced by `build_context` skipping `return` only when `into` is None in
`crates/steer-core/src/template.rs`, and the tests `render_bare_task` and
`assigned_value_call_renders_set_prompt_with_target`.

#### Scenario: a bare call suppresses return

- **WHEN** a bare call provides a return argument
- **THEN** return is not bound and the rendered instruction carries no set prompt.

#### Scenario: an assigned call exposes return

- **WHEN** an assigned call provides a return argument
- **THEN** return is bound to the context and the rendered instruction carries a set prompt.

### Requirement: Argument values use runtime scope with a literal fallback

When a runtime variable scope is available the renderer SHALL evaluate each argument
expression against that scope; when no scope is available (static or simulation rendering) it
SHALL degrade to static literal evaluation. A render-time evaluation failure against the
runtime scope SHALL fall back to the static literal, never raise a render error. Evidenced by
`arg_value` and `build_context` in `crates/steer-core/src/template.rs`, the live path in
`crates/steer-core/src/vm.rs`, and the static path in `crates/steer-core/src/simulate.rs`.

#### Scenario: the static path uses literal evaluation

- **WHEN** a call is rendered during simulation with no runtime scope
- **THEN** argument expressions are evaluated as static literals.

#### Scenario: a runtime evaluation failure falls back to a literal

- **WHEN** an argument expression cannot be evaluated against the runtime scope
- **THEN** the renderer falls back to the static literal value instead of erroring.

### Requirement: Single-brace placeholders are preserved as literals

The renderer SHALL preserve single-brace placeholders in instruction text as literal output,
because a single brace pair is not Jinja2 syntax and is never interpreted as interpolation.
A call whose instruction argument contains a single-brace placeholder SHALL render that
placeholder verbatim. Evidenced by `build_context`/`render_call` in
`crates/steer-core/src/template.rs` and the test
`interpolation_in_instruction_preserved_as_placeholder`.

#### Scenario: a single-brace placeholder survives rendering

- **WHEN** a call's instruction argument contains a single-brace placeholder
- **THEN** the rendered instruction contains that placeholder unchanged.
