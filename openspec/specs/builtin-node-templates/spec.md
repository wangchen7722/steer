# Built-in Node Templates

## Purpose

Define the shipped template content for the six default action nodes (task, ask, command,
collect, print, judge), their distinct value-source opening semantics, the value-return,
intrinsic-boolean, and no-value behaviors, the produce block rendering, and the standard
schema and output-section conventions workflow template sets follow.

## Requirements

### Requirement: Six default nodes ship with distinct opening semantics

The shipped default node set SHALL comprise exactly six nodes (task, ask, command, collect,
print, judge), each rendered from a template whose opening wording conveys a distinct
value-source semantic so a caller can tell value sources apart in rendered text. The collect
node SHALL convey a self-investigation reasoning semantic (the agent does the work itself),
which SHALL be meaningfully distinct from the ask node (ask the human user) and the command
node (run a shell command). Evidenced by the six built-in body constants and the
`.steer/templates/default/*.j2.md` files in `crates/steer-core/src/template.rs`, and the tests
`value_nodes_differentiate_by_source` and `collect_template_conveys_reasoning_semantic`.

#### Scenario: collect reads as self-investigation, not asking or shell

- **WHEN** a collect call is rendered
- **THEN** its wording conveys self-investigation and does not read like ask-the-user or
  run-a-shell-command.

#### Scenario: ask reads as asking the user

- **WHEN** an ask call is rendered
- **THEN** its wording directs the agent to ask the human user for the value.

### Requirement: Value nodes emit a set prompt toward a known target

A value node (task, ask, command, collect) with a return argument and a known receiver SHALL
emit a value-return set prompt in the rendered instruction that names the assignment target
as the variable the agent sets. A bare value call (no receiver) SHALL NOT emit such a prompt.
Evidenced by the built-in body constants in `crates/steer-core/src/template.rs` and the test
`assigned_value_call_renders_set_prompt_with_target`.

#### Scenario: an assigned value call emits a set prompt naming the target

- **WHEN** a value node call with a return argument is assigned to a variable
- **THEN** the rendered instruction contains a set prompt naming that variable.

#### Scenario: a bare value call emits no set prompt

- **WHEN** a value node call with a return argument is used as a bare statement
- **THEN** the rendered instruction contains no set prompt.

### Requirement: judge emits a boolean and print emits no value

The judge node SHALL be an intrinsic-boolean node that emits a true-or-false answer prompt of
the form `steer instance set <instance> <target> true` (or false) and carries no return
argument. The print node SHALL be a no-value node (return kind none) that emits no set prompt
and no verification. Evidenced by the JUDGE and PRINT body constants and the return kinds in
`fallback_template` in `crates/steer-core/src/template.rs`, and the test
`render_judge_asks_for_boolean_and_targets_var`.

#### Scenario: judge prompts for a boolean toward its target

- **WHEN** a judge call is assigned to a variable
- **THEN** the rendered instruction prompts for true or false and names the target variable.

#### Scenario: print emits no value prompt

- **WHEN** a print call is rendered
- **THEN** the rendered instruction contains no set prompt and no verification.

### Requirement: A produce list renders a produce block

A call that provides a non-empty produce list SHALL render a produce block that lists each
file, introduced by wording directing the agent to write or update the listed files as part of
the work. Evidenced by the shared produce block in the built-in body constants in
`crates/steer-core/src/template.rs`.

#### Scenario: a produce list renders a file block

- **WHEN** a call provides a produce list of two files
- **THEN** the rendered instruction contains a produce block listing both files.

### Requirement: Workflow sets follow a standard schema and output convention

A workflow template set SHALL declare a standard schema with a required instruction string
parameter, and most SHALL carry an on_check verification template. The per-workflow template
files under a workflow template directory embed a template output section with HTML-comment
placeholders the agent fills (and which are removed from final output), and encode
workflow-specific output-language rules as rules blocks. Evidenced by the workflow template
files under
`.steer/templates/{openspec-generate-specs,openspec-superpowers,os-bugfix}/` and the parser in
`crates/steer-core/src/template.rs`.

#### Scenario: a workflow template declares the standard instruction parameter

- **WHEN** a workflow template file is parsed
- **THEN** its schema includes a required instruction string parameter.

#### Scenario: a workflow template embeds an output section with placeholders

- **WHEN** a workflow template body is rendered
- **THEN** it contains a template output section whose HTML-comment placeholders are intended
  to be filled and then removed from the final output.
