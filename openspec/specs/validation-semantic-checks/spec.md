# Validation Semantic Checks

## Purpose

The load-time static validator that runs on a parsed `Module` and returns a single
`Vec<Diagnostic>` containing every error, so authors see all problems at once. It is
pure, idempotent, and filesystem-free. This capability covers shape and placement rules
only; it does NOT enforce declared-return-type value semantics, which migrated to runtime
`set`-time and belongs to
[runtime-check-gate](../runtime-check-gate/spec.md).

## Requirements

### Requirement: The validator returns all errors in a single pass

The single public entry `validate(module: &Module) -> Vec<Diagnostic>` SHALL produce
every diagnostic in one pass and MUST NOT stop at the first error. Every diagnostic SHALL
carry a source `Span` and a human-readable message, and all v1 diagnostics SHALL be
`Severity::Error` (no warn/error distinction). Authors MUST NOT rely on diagnostic
message text as a stable interface. Evidenced by `pub fn validate`, `enum Severity`, and
`struct Diagnostic` in `crates/steer-core/src/validate.rs`, and the test
`multiple_diagnostics_reported`.

#### Scenario: multiple problems are reported together

- **WHEN** a workflow contains more than one distinct semantic error
- **THEN** `validate` returns a `Vec<Diagnostic>` containing one entry per problem rather
  than stopping at the first.

#### Scenario: every diagnostic is an error

- **WHEN** `validate` reports any problem
- **THEN** the diagnostic's severity is `Severity::Error`, the diagnostic carries a
  source `Span`, and a human-readable message is attached.

### Requirement: A no-value node used as a value is rejected

The validator SHALL reject, via `check_value_expr`, a node call used in an assignment or `return` position
when the callee produces no value (`return_spec().kind == ParamKind::None`, e.g. `print`,
or a callee with no resolvable template), with a message stating the call produces no
value and cannot be assigned or returned. Evidenced by `check_value_expr` and the test
`print_cannot_be_assigned`.

#### Scenario: a print call cannot be assigned

- **WHEN** a workflow uses a no-value call such as `print` as the right-hand side of an
  assignment or as a `return` value
- **THEN** `validate` emits a diagnostic reporting that the call produces no value and
  cannot be assigned or returned.

### Requirement: A String-return value node needs a `return=` format argument

The validator SHALL reject a node call whose declared return kind is `ParamKind::String`
(`task`, `ask`, `command`, `collect`) when used as a value (assignment target or `return`) without
a `return=` named argument, with a message stating the value node needs a `return=` format
argument. This is a load-time shape rule only; the RUNTIME enforcement of the declared
return type's value semantics happens at `set`-time in
[runtime-check-gate](../runtime-check-gate/spec.md). Evidenced by `check_value_expr` and
the tests `value_task_assigned_without_return_is_error` and
`return_of_value_task_needs_return_arg`.

#### Scenario: a String-return node assigned without `return=` is rejected

- **WHEN** a workflow assigns the result of a String-return node call such as
  `task(...)` without supplying a `return=` named argument
- **THEN** `validate` emits a diagnostic reporting that the value node needs a `return=`
  format argument.

#### Scenario: a String-return node returned without `return=` is rejected

- **WHEN** a workflow uses a String-return node call as a `return` value without a
  `return=` named argument
- **THEN** `validate` emits a diagnostic reporting that the value node needs a `return=`
  format argument.

### Requirement: IntrinsicBool- and List-return value nodes need no `return=`

The validator SHALL accept a value node call whose declared return kind is
`ParamKind::IntrinsicBool` (e.g. `judge`) or `ParamKind::List` in an assignment or
`return` position without a `return=` named argument. Evidenced by `check_value_expr` and
the test `judge_assigned_without_return_is_ok`.

#### Scenario: a judge call is assigned without `return=`

- **WHEN** a workflow assigns the result of an `IntrinsicBool`-return call such as
  `judge(...)` without a `return=` named argument
- **THEN** `validate` emits no diagnostic for that call.

### Requirement: A user-function call used as a value needs no `return=`

A call whose callee resolves to a user-defined function SHALL be accepted in an assignment
or `return` position without a `return=` named argument, because user functions always
produce a value. Evidenced by `check_value_expr` and the test
`user_func_assigned_is_ok_without_return_arg`.

#### Scenario: a user-function result is assigned without `return=`

- **WHEN** a workflow assigns the result of a user-defined function call without a
  `return=` named argument
- **THEN** `validate` emits no diagnostic for that call.

### Requirement: A bare side-effect call needs no `return=`

A node call used as a bare statement (not assigned, not returned) SHALL be valid without a
`return=` named argument, because it is used only for its side effects. Evidenced by
`check_value_expr` and the test `bare_task_without_return_is_ok`.

#### Scenario: a bare task call without `return=` is accepted

- **WHEN** a workflow invokes a node call as a bare statement with no receiver and no
  `return=` named argument
- **THEN** `validate` emits no diagnostic for that call.

### Requirement: Reserved binding names are rejected at binding sites

`check_binding` / `is_reserved_binding` SHALL reject a binding target named `checked`
(the per-op verification flag) or any name starting with `__` (hidden loop/return slots),
with a message stating the name is reserved and cannot be bound. The check SHALL apply to
all three binding forms — `Assign` targets, `For.var` loop variables, and `Function.params`
— and SHALL NOT apply to value expressions (free-reading a `__`-name or `checked` as a
value is not a violation). Evidenced by `is_reserved_binding`, `check_binding`, and the
test `reserved_binding_names_are_rejected`.

#### Scenario: `checked` is rejected as a binding target

- **WHEN** a workflow binds the name `checked` as an assignment target, a loop variable,
  or a function parameter
- **THEN** `validate` emits a diagnostic reporting that `checked` is a reserved name and
  cannot be bound.

#### Scenario: a `__`-prefixed name is rejected as a binding target

- **WHEN** a workflow binds a name beginning with `__` as an assignment target, a loop
  variable, or a function parameter
- **THEN** `validate` emits a diagnostic reporting that the name is reserved and cannot be
  bound.

#### Scenario: reading a reserved name as a value is not a violation

- **WHEN** a workflow reads `checked` or a `__`-prefixed name as a value in an expression
  rather than binding it
- **THEN** `validate` emits no reserved-name diagnostic for that use.

### Requirement: Functions must be defined at the top level

`collect_funcs` SHALL reject a `func` nested inside another statement body (`if`, `loop`,
`for`, or another `func`) with a message stating functions must be defined at the top
level. Evidenced by `collect_funcs` and its `top_level` guard in
`crates/steer-core/src/validate.rs`.

#### Scenario: a nested function is rejected

- **WHEN** a workflow defines a `func` inside the body of an `if`, `loop`, `for`, or
  another `func`
- **THEN** `validate` emits a diagnostic reporting that functions must be defined at the
  top level.

### Requirement: Duplicate top-level function definitions are rejected

`collect_funcs` SHALL reject a second top-level function definition that reuses an
existing function name, with a message naming the duplicated function. Evidenced by
`collect_funcs` and the test `duplicate_function`.

#### Scenario: two functions with the same name

- **WHEN** a workflow defines two top-level functions with the same name
- **THEN** `validate` emits a diagnostic reporting the duplicate function `<name>`.

### Requirement: Duplicate function parameters are rejected

`collect_funcs` SHALL reject a function whose parameter list contains a repeated name,
with a message naming the duplicated parameter and the enclosing function. Evidenced by
`collect_funcs` and the test `duplicate_parameter`.

#### Scenario: a function with two parameters sharing a name

- **WHEN** a workflow defines a function whose parameter list contains the same name more
  than once
- **THEN** `validate` emits a diagnostic reporting the duplicate parameter `<p>` in
  function `<name>`.

### Requirement: A duplicate named argument in a call is rejected

`check_call` SHALL reject a call that supplies the same named argument more than once,
with a message naming the duplicated argument, and this check SHALL run before any
formatter-driven argument checks. Positional arguments are NOT deduplicated. Evidenced by
`check_call` and the test `duplicate_named_argument`.

#### Scenario: the same named argument twice

- **WHEN** a call supplies the same named argument name more than once
- **THEN** `validate` emits a diagnostic reporting the duplicate argument `<name>`.

#### Scenario: repeated positional arguments are not deduplicated

- **WHEN** a call supplies the same positional value more than once
- **THEN** `validate` does not emit a duplicate-argument diagnostic for the positional
  values.

### Requirement: User-function calls skip formatter-driven argument checks

For a call whose callee is a user-defined function, `check_call` SHALL short-circuit on
`funcs.contains_key` and skip all formatter-driven argument checks (required-positional,
required-named, and named-argument type checks). User-function argument binding is handled
by the `Call` IR instruction at runtime, not by this validator. Evidenced by the
`funcs.contains_key` short-circuit in `check_call`.

#### Scenario: a user-function call skips argument checks

- **WHEN** a workflow calls a user-defined function with arbitrary arguments
- **THEN** `validate` skips the formatter-driven required-positional, required-named, and
  named-argument type checks for that call.

### Requirement: A required `instruction` positional argument must be supplied

`check_call` SHALL reject a node call that declares a required `instruction` parameter
(the first positional) but supplies no positional argument, with a message stating the
callee requires an instruction argument. All built-in nodes declare `instruction` as
required. Evidenced by `check_call` and the built-in param specs in
`crates/steer-core/src/template.rs`.

#### Scenario: a node call missing its instruction positional

- **WHEN** a workflow calls a built-in node that declares a required `instruction`
  parameter without supplying any positional argument
- **THEN** `validate` emits a diagnostic reporting that `<callee>` requires an instruction
  argument.

### Requirement: Required named arguments must be present

`check_call` SHALL reject a node call that omits a required named parameter (any param
with `required: true` other than `instruction`), with a message naming the callee and the
missing parameter. Nodes such as `ask`, `command`, and `collect` declare `return` as
required. Evidenced by `check_call` and the built-in param specs in
`crates/steer-core/src/template.rs`.

#### Scenario: a missing required named argument

- **WHEN** a workflow calls a node that declares a required named parameter (e.g. `return`
  on `ask`/`command`/`collect`) without supplying that named argument
- **THEN** `validate` emits a diagnostic reporting that `<callee>` requires a `<param>`
  argument.

### Requirement: Unknown named arguments are silently accepted

`check_call` SHALL NOT emit a diagnostic for a named argument that does not match any
declared parameter of the callee, to remain forward-compatible with parameters added by
future node versions. Evidenced by `check_call`, which performs no unknown-named-argument
rejection.

#### Scenario: an unrecognized named argument is accepted

- **WHEN** a workflow supplies a named argument that the callee does not declare
- **THEN** `validate` emits no diagnostic for that argument.

### Requirement: A String-declared named argument must be a string literal

`check_call` SHALL reject a named argument whose declared kind is `ParamKind::String` when
the argument value is not an `Expr::String`, with a message stating the argument must be a
string literal. Only `Expr::String` passes this check. Evidenced by `check_call` and the
test `check_must_be_string`.

#### Scenario: a non-string value for a String-declared named argument

- **WHEN** a workflow supplies a non-string value to a named argument declared as
  `ParamKind::String`
- **THEN** `validate` emits a diagnostic reporting that argument `<name>` of `<callee>`
  must be a string literal.

### Requirement: A Bool-declared named argument must be a `true`/`false` reference

`check_call` SHALL reject a named argument whose declared kind is `ParamKind::Bool` when
the argument value is not a `true` or `false` bareword variable reference, with a message
stating the argument must be a boolean. Boolean literals are represented as bareword
variable references, not as a dedicated AST node. Evidenced by `check_call`.

#### Scenario: a non-boolean value for a Bool-declared named argument

- **WHEN** a workflow supplies a value other than a `true` or `false` bareword reference
  to a named argument declared as `ParamKind::Bool`
- **THEN** `validate` emits a diagnostic reporting that the argument must be a boolean
  (`true` or `false`).

### Requirement: A List-declared named argument must be a list literal

`check_call` SHALL reject a named argument whose declared kind is `ParamKind::List` when
the argument value is not an `Expr::List`, with a message stating the argument must be a
list literal. Only `Expr::List` passes this check. Evidenced by `check_call` and the test
`produce_must_be_list`.

#### Scenario: a non-list value for a List-declared named argument

- **WHEN** a workflow supplies a non-list value to a named argument declared as
  `ParamKind::List` (for example the `produce` argument of `collect`)
- **THEN** `validate` emits a diagnostic reporting that the argument must be a list
  literal.

### Requirement: Meta `return=` arguments are skipped by named-argument type checking

`check_call` SHALL skip the `return` named argument (and any `none`/`bool` meta form)
during named-argument type checking. The validator SHALL NOT statically type-check the
value of a `return=` argument: the String-return case is covered by the value-node
`needs a return=` shape rule, and the declared return type's RUNTIME value enforcement
happens at `set`-time in [runtime-check-gate](../runtime-check-gate/spec.md). Evidenced
by `check_call` excluding meta args from the type matrix, and the test
`return_arg_must_be_string` confirming the `return` value is itself treated as a String
argument only where applicable rather than via the type matrix.

#### Scenario: the `return=` argument is not type-checked by the named-arg matrix

- **WHEN** a workflow supplies a `return=` named argument to a value node call
- **THEN** `validate` does not type-check that argument via the named-argument type matrix
  and does not emit a type-matrix diagnostic for it.

### Requirement: No type check on positional or missing named arguments

`check_call` SHALL NOT perform a type check on positional argument values, and SHALL NOT
perform a type check on a named argument that is absent. Type checks apply only to named
arguments that are actually present. Evidenced by `check_call` applying the type matrix
solely to present named arguments.

#### Scenario: positional argument values are not type-checked

- **WHEN** a workflow supplies positional argument values of any expression form to a call
- **THEN** `validate` does not emit a type-matrix diagnostic for the positional values.

#### Scenario: missing named arguments are not type-checked

- **WHEN** a workflow omits a named argument that is not declared required
- **THEN** `validate` does not emit a type-matrix diagnostic for the absent argument.

### Requirement: The validator traverses into nested blocks and expression positions

The validator SHALL recurse into `if`/`elseif`/`else`, `loop ... until`, `for ... in`,
and function bodies to apply the value-expression, binding, and call-argument rules
everywhere, and SHALL traverse expression positions — binary operands, unary operands,
list elements, and string interpolations — applying `check_call` and the type rules to
each sub-expression. Evidenced by `visit_block` / `visit_stmt` / `visit_expr` and the test
`nested_blocks_are_checked`.

#### Scenario: errors inside nested blocks are reported

- **WHEN** a workflow places a semantic violation inside the body of an `if`, `loop`, `for`,
  or a function body
- **THEN** `validate` still detects and reports the violation.

#### Scenario: sub-expressions are checked

- **WHEN** a workflow embeds a call inside a binary operand, unary operand, list element,
  or string interpolation
- **THEN** `validate` applies the call-argument and type rules to that sub-expression.

### Requirement: The validator SHALL NOT perform variable-resolution or target checks

The validator SHALL NOT perform an undefined-variable check: reading an undeclared name is
accepted. The validator SHALL NOT perform a step-target or missing-target check, and SHALL
NOT perform a duplicate-step-name check. These are intentional non-contracts; this
capability covers static shape and placement only. Evidenced by the absence of any
variable-resolution or step-target logic in `crates/steer-core/src/validate.rs`.

#### Scenario: an undefined variable read is accepted

- **WHEN** a workflow reads a name that was never declared or assigned
- **THEN** `validate` emits no diagnostic for the read.

#### Scenario: a missing or duplicate step target is not validated

- **WHEN** a workflow references a step target that does not exist or defines duplicate
  step names
- **THEN** `validate` emits no diagnostic for the target, because target checks are not
  part of this validator.
