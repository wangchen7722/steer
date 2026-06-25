# Template Substitution Syntax

## Purpose

Define the minimal Jinja2-subset template language steer renders node bodies and
`on_check` blocks with: exactly `{{ name }}` interpolation, `{% if %}/{% else %}/{% endif %}`,
and `{% for x in list %}/{% endfor %}`, where every operand is a single bare variable name,
plus the truthiness, missing-variable, and parse-error rules the renderer guarantees.

## Requirements

### Requirement: Exactly three syntax forms are supported

The renderer SHALL support exactly three syntax forms and nothing richer: `{{ name }}`
interpolation, `{% if name %}…{% else %}…{% endif %}` (the `{% else %}` branch optional),
and `{% for item in list %}…{% endfor %}`. The condition operand of `{% if %}` and both
operands of `{% for %}` (the bound item name and the iterable name) SHALL each be a single
bare variable name obtained by trimming the tag text; filters, conditional expressions,
whitespace control (`{%-`/`-%}`), and comments (`{# #}`) are out of scope and SHALL NOT be
interpreted. Evidenced by `Node::{Var, If, For}` and `build` in
`crates/steer-core/src/template.rs`.

#### Scenario: interpolation renders a bound value

- **WHEN** a template contains `hi {{ name }}` and `name` is bound to `bob`
- **THEN** the rendered output is `hi bob`.

#### Scenario: an if/else selects a branch

- **WHEN** a template contains `{% if flag %}ON{% else %}OFF{% endif %}`
- **THEN** it renders `ON` when `flag` is true and `OFF` when `flag` is false.

#### Scenario: a for loop iterates a list

- **WHEN** a template contains `{% for f in files %}[{{ f }}] {% endfor %}` and `files` is
  a list of `a`, `b`
- **THEN** the rendered output is `[a] [b] `.

### Requirement: A missing variable is empty or falsy, never an error

The renderer SHALL treat a variable absent from the render context as the Null
value in both interpolation and conditional evaluation, rendering it as the empty
string and evaluating it as falsy. The renderer SHALL NOT raise an error for a
missing variable. A for-loop over a non-list value (a missing key, Null, a string,
a number, or an object) SHALL yield zero iterations and SHALL NOT raise an error.
Evidenced by `render_nodes` using `ctx.get(name).unwrap_or(&Value::Null)` in
`crates/steer-core/src/template.rs` and the test `for_over_missing_renders_nothing`.

#### Scenario: missing interpolation renders empty

- **WHEN** a template `hi {{ name }}` is rendered with no `name` bound
- **THEN** the output is `hi ` with no error.

#### Scenario: a for over a missing or non-list value yields no iterations

- **WHEN** a template `{% for f in files %}{{ f }}{% endfor %}end` is rendered where `files`
  is missing or is a scalar
- **THEN** the output is `end` with no error.

### Requirement: Truthiness treats zero as true

The renderer SHALL evaluate truthiness as: `Null` is false; `Bool` is its own value; `Int`
and `Float` are always true, including zero; `Str`, `List`, and `Object` are true when
non-empty and false when empty. This SHALL be the truthiness used by `{% if %}` conditions.
Evidenced by `Value::truthy` in `crates/steer-core/src/value.rs`.

#### Scenario: a numeric zero is truthy

- **WHEN** an `{% if n %}` condition is evaluated with `n` bound to the integer `0`
- **THEN** the condition is true (the then-branch renders).

#### Scenario: an empty string or list is falsy

- **WHEN** an `{% if s %}` condition is evaluated with `s` bound to an empty string or an
  empty list
- **THEN** the condition is false (the else-branch, or nothing, renders).

### Requirement: A for loop binds the item in a flat cloned scope

A `{% for item in list %}` loop SHALL bind the loop item by inserting it into a flat clone
of the context map for each iteration. If the bound item name already exists in the context,
it SHALL be overwritten for the loop body and SHALL NOT be restored after the loop ends.
Evidenced by `render_nodes` cloning `ctx` and `sub.insert(var.clone(), item.clone())` in
`crates/steer-core/src/template.rs`.

#### Scenario: the loop variable shadows an existing key for the loop body

- **WHEN** a for loop binds `x` over a list and the context already has an `x`
- **THEN** the loop body sees the loop item, not the original `x`, during iteration.

### Requirement: Parse errors are distinct stable variants

Parsing SHALL fail with one of these distinct, stable error variants:
`UnterminatedExpr` (a `{{ …` never closed), `UnterminatedTag` (a `{% …` never closed),
`WrongClose { expected, found }` (a block closed by the wrong tag, or an `if`/`for` reaching
end of template without its closer), `UnmatchedClose` (a closing tag with no opener), and
`MalformedFor` (a `for` tag without ` in ` separating the variable and iterable). An `if`
block SHALL be closed by an optional `{% else %}` then `{% endif %}`; a `for` block SHALL be
closed by `{% endfor %}`. A mismatched closer is an error, not a fallback. Evidenced by
`enum TemplateError` and `build` in `crates/steer-core/src/template.rs`, and the tests
`err_unterminated_expr`, `err_wrong_close`, `err_unmatched_close`.

#### Scenario: an unterminated expression errors

- **WHEN** a template `hi {{ name` (no closing `}}`) is parsed
- **THEN** parsing fails with `UnterminatedExpr`.

#### Scenario: a wrong closing tag errors

- **WHEN** a template `{% for x in xs %}{% endif %}` is parsed
- **THEN** parsing fails with `WrongClose { expected: "endfor", .. }`.

#### Scenario: an unmatched close errors

- **WHEN** a template `hello {% endif %}` is parsed
- **THEN** parsing fails with `UnmatchedClose`.
