# Templates And Instruction Rendering

> Behavior specs for the template engine: Jinja2 subset, `@template` directory selection, fallback order, persistence across resume, and target-aware value-return prompts.

## Scenario: Jinja-style interpolation, if, and for
- **WHEN** a template uses `{{ name }}`, `{% if %}/{% else %}/{% endif %}`, or
  `{% for x in list %}/{% endfor %}`
- **THEN** it renders against the call arguments and runtime values.

## Scenario: workflow template directory selection
- **WHEN** `@template = "planning"` executes before an action node
- **THEN** subsequent action nodes first resolve templates from
  `.steer/templates/planning/<callee>.j2.md`.

## Scenario: template fallback order
- **WHEN** an action node is rendered
- **THEN** resolution checks the active template directory, then
  `.steer/templates/default/<callee>.j2.md`, then the built-in template.

## Scenario: template selection persists across resume
- **WHEN** a workflow changes `@template` and the context is serialized
- **THEN** `context.json` preserves `meta.template_dir` and resumed execution
  uses the same active template directory.

## Scenario: step instructions do not include check mechanics
- **WHEN** `step` renders an action node with `check=`
- **THEN** the task instruction is rendered without the verification prompt;
  verification is rendered by `check`.

## Scenario: value return prompt is target-aware
- **WHEN** `x = task("...", return="...")` is rendered
- **THEN** the instruction tells the agent to set `x`.
- **WHEN** a bare `task("...", return="...")` is rendered
- **THEN** the instruction does not render a `steer instance set <name> <var>` prompt.

## Scenario: runtime interpolation is preserved for simulation
- **WHEN** simulation renders an instruction containing `{f}` for a runtime
  variable
- **THEN** the rendered instruction keeps `{f}` as a placeholder.
