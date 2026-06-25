# Template File Format

## Purpose

Define the `.j2.md` template file format authors write: an optional YAML-style front matter
declaring the node parameter schema and an optional `on_check` verification template, the
recognized parameter types and modifiers, the engine-level `return` semantics, and the
on_check body contract that excludes the auto-appended report section.

## Requirements

### Requirement: A file optionally begins with front matter

A template file SHALL either begin with a front-matter block delimited by an opening line of
exactly three dashes and a closing line of three dashes, or be treated as having no front
matter. When front matter is present, the text between the delimiters is the parameter schema
and the text after the closing delimiter is the body. When front matter is absent, the entire
file SHALL be the body and the parsed schema SHALL be a single required instruction string
parameter. Evidenced by `split_front_matter` and `parse_template` in
`crates/steer-core/src/template.rs`.

#### Scenario: a file without front matter gets a minimal schema

- **WHEN** a template file has no leading three-dash line
- **THEN** the whole file is the body and the schema is one required instruction string
  parameter.

#### Scenario: a file with front matter splits schema and body

- **WHEN** a template file begins with a three-dash line, schema lines, a closing three-dash
  line, then body text
- **THEN** the text before the closing delimiter is the schema and the text after is the body.

### Requirement: Parameter lines declare name, type, and modifiers

Outside an on_check block, the parser SHALL treat each non-empty front-matter line that is
not the parameter header as a parameter spec of the form: a name, a colon, a type token, and
optional modifiers. The recognized parameter type tokens SHALL be the four scalar and
collection kinds (string, bool, list, none), plus the extended boolean token that maps to the
intrinsic-boolean kind for a return parameter. A type token that is not one of the recognized
values SHALL cause that line to be silently skipped with no error. A required modifier SHALL
mark a parameter required, and a default modifier SHALL provide a declarative default value.
Evidenced by `parse_front_matter` in `crates/steer-core/src/template.rs`.

#### Scenario: a recognized type produces a parameter

- **WHEN** a front-matter line is `instruction: string, required`
- **THEN** a required string parameter named instruction is added to the schema.

#### Scenario: an unrecognized type is skipped

- **WHEN** a front-matter line declares a type that is not string, bool, list, or none
- **THEN** the line is silently skipped and no parameter is added.

### Requirement: Defaults accept only booleans and quoted strings

The `default=` modifier SHALL parse only the literal `true`, the literal `false`, and a
double-quoted string literal as a default value. Any other default form SHALL parse to no
default (None). Evidenced by `parse_default` in `crates/steer-core/src/template.rs`.

#### Scenario: a quoted string default is parsed

- **WHEN** a default modifier is `default="output"`
- **THEN** the parameter's default is the string value `output`.

#### Scenario: an unquoted non-boolean default yields no default

- **WHEN** a default modifier is an unquoted bareword that is not true or false
- **THEN** the parameter carries no default value.

### Requirement: The return parameter carries engine-level semantics

A parameter named `return` with type `none` SHALL mark a node that produces no value and
therefore cannot be assigned or returned. A parameter named `return` with the intrinsic-boolean
type SHALL mark a node that returns a boolean without needing a return argument and is
assignable directly. The validation logic and the VM SHALL rely on these hardcoded return-kind
semantics. Evidenced by `ParamKind::{None, IntrinsicBool}` and `NodeTemplate::return_spec` in
`crates/steer-core/src/template.rs`.

#### Scenario: a return-none node cannot carry a value

- **WHEN** a template declares `return: none`
- **THEN** the node is marked as producing no value and a call to it cannot be assigned or
  returned.

#### Scenario: an intrinsic-boolean node is directly assignable

- **WHEN** a template declares its return parameter as the intrinsic-boolean kind
- **THEN** the node returns a boolean without a return argument and may be assigned directly.

### Requirement: on_check is a Jinja2 template the report section never inhabits

An `on_check:` key SHALL accept either an inline single value (`on_check: text`) or a YAML
literal block introduced by `on_check: |` whose subsequent indented lines are collected as the
template body. The on_check template SHALL be parsed and rendered by the same Jinja2 engine as
node bodies, against a context that includes the evaluated check value exposed as the check
variable. The `<report>` verification section SHALL NEVER appear in an on_check body; it is
always auto-appended by the VM. A template without an on_check SHALL degrade to a plain-text
rendering of the evaluated check value at check time. Evidenced by `parse_front_matter`,
`render_check`, and the `on_check` field of `NodeTemplate` in
`crates/steer-core/src/template.rs`, and the tests `parse_front_matter_with_on_check_block`
and `render_check_fallback_without_on_check`.

#### Scenario: an on_check block template is collected and rendered

- **WHEN** a front matter contains an `on_check: |` block with indented Jinja2 lines
- **THEN** those lines are collected as the on_check template and rendered against the check
  context at check time.

#### Scenario: a template without on_check falls back to plain text

- **WHEN** a template has no on_check and a call provides a check value
- **THEN** the rendered check instruction is the plain-text rendering of that check value.
