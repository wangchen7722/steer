# Value System and Operators

## Purpose

The typed runtime value system `Value` (`Null | Bool | Int | Float | Str | List |
Object`) carries all dynamic state in steer, plus the operator-semantic table that
`eval` (see [interpreter-execution](openspec/specs/interpreter-execution/spec.md))
applies and the typed-JSON coercion `parse_value` that backs the
`steer set` value contract (see
[runtime-check-gate](openspec/specs/runtime-check-gate/spec.md)). It defines what
workflow authors can compute at runtime and what shapes an agent may store.

## Requirements

### Requirement: Truthiness follows per-type rules with zero as true

The runtime SHALL define truthiness for control flow (`if`/`until`, see
[interpreter-execution](openspec/specs/interpreter-execution/spec.md)) as:
`Null` to false; `Bool(b)` to `b`; any `Int` or `Float` (including zero) to true;
`Str` to non-empty; `List` to non-empty; `Object` to non-empty. Zero SHALL be
truthy, so an integer step counter or count drives a loop until it is removed,
not until it crosses zero. Evidenced by `Value::truthy` in
`crates/steer-core/src/value.rs`.

#### Scenario: zero and non-zero numbers are both true

- **WHEN** a control condition evaluates an `Int(0)` or an `Int(5)`
- **THEN** both are truthy.

#### Scenario: empty containers and null are false

- **WHEN** a control condition evaluates `Null`, an empty `Str`, an empty `List`,
  or an empty `Object`
- **THEN** all are falsy.

### Requirement: Integer arithmetic is checked and integer division truncates

Arithmetic (`+`, `-`, `*`, `/`) SHALL require numeric operands. `Int op Int`
SHALL yield `Int` with checked overflow (overflow yields
`EvalError::Arithmetic("integer overflow")`); `x / 0` SHALL yield
`Arithmetic("division by zero")`; integer division SHALL truncate toward zero so
`7 / 2` is `3`. If either operand is `Float`, both operands SHALL be promoted to
`f64` and the result SHALL be `Float` (so `7.0 / 2` is `3.5`). A non-number
operand SHALL yield `TypeError`. Evidenced by `apply_binop` and the tests
`eval_arithmetic`, `eval_div_by_zero` in `crates/steer-core/src/value.rs`.

#### Scenario: integer division truncates toward zero

- **WHEN** `7 / 2` is evaluated with both operands as `Int`
- **THEN** the result is `Int(3)`.

#### Scenario: float operands produce a float result

- **WHEN** `7.0 / 2` is evaluated with the left operand as `Float`
- **THEN** the result is `Float(3.5)`.

#### Scenario: division by zero and overflow raise Arithmetic errors

- **WHEN** an integer operand is divided by `0`, or an `Int op Int` overflows
- **THEN** evaluation returns `EvalError::Arithmetic`.

### Requirement: Equality of mismatched non-number types is a TypeError

Equality (`==`, `!=`) SHALL compare same-type values directly; two numbers SHALL
be compared numerically; `Null == Null` SHALL be true. A comparison between
mismatched non-number types (e.g. `1 == "1"`) SHALL raise `TypeError` rather than
silently returning false. Ordering (`<`, `>`, `<=`, `>=`) SHALL be numeric for
numbers (mixed via `f64`), lexicographic for same-type strings, and SHALL raise
`TypeError("cannot compare X and Y")` for mismatched non-number types. Evidenced
by `values_eq` / `compare` in `crates/steer-core/src/value.rs`.

#### Scenario: a number and a string compared for equality raise TypeError

- **WHEN** `1 == "1"` is evaluated
- **THEN** evaluation returns `EvalError::TypeError` rather than `Bool(false)`.

#### Scenario: two nulls are equal

- **WHEN** `Null == Null` is evaluated
- **THEN** the result is `Bool(true)`.

#### Scenario: ordering mismatched non-number types raises TypeError

- **WHEN** a `Str` is ordered against an `Object`
- **THEN** evaluation returns `EvalError::TypeError`.

### Requirement: Logical and/or short-circuit and always return Bool

Logical `and` SHALL, in `eval`, evaluate its left operand first and return
`Bool(false)` without evaluating the right operand when the left is falsy. Logical
`or` SHALL return `Bool(true)` without evaluating the right operand when the left
is truthy. The result of `and`/`or` SHALL always be `Bool`, never the operand
value. `not` SHALL return `Bool(!v.truthy())`. Unary `-` SHALL checked-negate an
`Int`, negate a `Float`, and raise `TypeError` on a non-number. Evidenced by the
`and`/`or` arms of `eval`, `apply_unop`, and the test
`eval_logical_short_circuits` in `crates/steer-core/src/value.rs`.

#### Scenario: and does not evaluate its right operand when left is falsy

- **WHEN** `false and <side effect>` is evaluated
- **THEN** the result is `Bool(false)` and the right operand is never evaluated.

#### Scenario: or returns true when its left operand is truthy

- **WHEN** `5 or <expression>` is evaluated
- **THEN** the result is `Bool(true)` and the right operand is never evaluated.

### Requirement: parse_value maps typed JSON to Value, else treats input as a string

`parse_value` SHALL attempt to parse its input as JSON; if parsing fails, the
entire input SHALL become `Value::Str`. Valid JSON SHALL map as: `null` to `Null`;
`bool` to `Bool`; a number with a whole `i64` form to `Int`, otherwise `Float`;
`string` to `Str`; `array` to `List` (recursively); `object` to `Object`. A
JSON-quoted string (e.g. `"[1, 2, 3]"`) SHALL become `Str`, and a bareword that
is not valid JSON (e.g. `hello`) SHALL become `Str`. This is the persisted-value
contract that `steer set` relies on. Evidenced by `parse_value`/`json_to_value`
and the test `parse_value_typed_literals` in `crates/steer-core/src/value.rs`.

#### Scenario: bare JSON literals map to their typed Value

- **WHEN** `parse_value` is given `42`, `true`, `"hi"`, or `[1,2,3]`
- **THEN** the results are `Int(42)`, `Bool(true)`, `Str("hi")`, and a `List` of
  three `Int` values respectively.

#### Scenario: invalid JSON and quoted strings become Str

- **WHEN** `parse_value` is given the bareword `hello` or the quoted string
  `"[1, 2, 3]"`
- **THEN** both results are `Value::Str` carrying the original input text.

### Requirement: Variable resolution and interpolation have fixed rules

`eval` SHALL resolve `Expr::Var("true")` to `Bool(true)` and `"false"` to
`Bool(false)`; any other name SHALL be looked up in the current variable scope,
and a missing name SHALL yield `EvalError::UnsetVar(name)`. `Expr::String(parts)`
SHALL concatenate literal runs with each evaluated interpolation rendered.
`Expr::Call` SHALL always yield `EvalError::UnexpectedCall` in `eval`, because
calls are separate instructions and never sub-expressions. Evidenced by the
`Var`/`String`/`Call` arms of `eval` in `crates/steer-core/src/value.rs`.

#### Scenario: a known literal name resolves without a binding

- **WHEN** `eval` resolves `Expr::Var("true")`
- **THEN** it returns `Bool(true)` regardless of the variable scope.

#### Scenario: a call used as a sub-expression is rejected

- **WHEN** `eval` encounters an `Expr::Call`
- **THEN** it returns `EvalError::UnexpectedCall`.

### Requirement: EvalError is a closed set of failure kinds

`EvalError` SHALL be the closed set `{ UnsetVar(String), TypeError(String),
Arithmetic(String), UnexpectedCall }`. Evidenced by `enum EvalError` in
`crates/steer-core/src/value.rs`.

#### Scenario: the error kinds are the four enumerated

- **WHEN** any runtime evaluation fails
- **THEN** the returned `EvalError` is one of `UnsetVar`, `TypeError`,
  `Arithmetic`, or `UnexpectedCall`.

### Requirement: Value::render formats each type as stable text

`Value::render` SHALL format each value type as deterministic text: `Null` to the
empty string, `Bool` to `true` or `false`, `Int` and `Float` to their decimal
forms, `Str` to its raw contents, `List` to the comma-and-space join of its
rendered elements, and `Object` to its JSON serialization. This is the text
convention that `{{ }}` interpolation and string interpolation rely on. Evidenced
by `Value::render` in `crates/steer-core/src/value.rs`.

#### Scenario: null renders empty and bool renders its word

- **WHEN** `Value::Null` and `Value::Bool(true)` are rendered
- **THEN** they format as the empty string and `true` respectively.

#### Scenario: a list renders its joined elements

- **WHEN** a `Value::List` of rendered elements is rendered
- **THEN** the result is the elements joined by a comma and a space.

### Requirement: Static literal evaluation degrades unknown names and compound expressions to placeholders

`eval_literal` SHALL, in the absence of a runtime scope, render an unknown
`Var(name)` (other than `true` or `false`) as the placeholder text `{name}`, and
SHALL render a `Binary`, `Unary`, or `Call` expression as the placeholder text
`{...}`. This static path is what the validator and the simulator use to render
instructions without resolving variables. Evidenced by `eval_literal` and
`interp_placeholder` in `crates/steer-core/src/value.rs`.

#### Scenario: an unknown variable degrades to a named placeholder

- **WHEN** `eval_literal` evaluates a `Var("count")` with no scope
- **THEN** the result is `Str("{count}")`.

#### Scenario: a compound expression degrades to an ellipsis placeholder

- **WHEN** `eval_literal` evaluates a `Binary` or `Call` expression with no scope
- **THEN** the result is `Str("{...}")`.

### Requirement: EvalError renders a stable Display string that becomes the halt reason

Each `EvalError` variant SHALL render a stable `Display` string: `UnsetVar(v)` to
a message naming the variable that is not set, `TypeError(m)` to a type-error
message, `Arithmetic(m)` to an arithmetic-error message, and `UnexpectedCall` to
a message stating that a call is unexpected in an expression. This text SHALL be
the halt reason surfaced to the agent through `Status::Halted`. Evidenced by
`impl Display for EvalError` in `crates/steer-core/src/value.rs`.

#### Scenario: an unset variable yields a naming message

- **WHEN** evaluation raises `EvalError::UnsetVar("count")`
- **THEN** its `Display` string names `count` as the variable that is not set.

### Requirement: A list literal evaluates each element and propagates the first error

`eval` SHALL evaluate an `Expr::List` by evaluating each element expression in
order and collecting the results into a `Value::List`. If any element evaluation
raises an `EvalError`, `eval` SHALL propagate that error immediately without
evaluating the remaining elements. Evidenced by the `Expr::List` arm of `eval` in
`crates/steer-core/src/value.rs`.

#### Scenario: a list literal evaluates to a list of values

- **WHEN** `eval` evaluates an `Expr::List` whose elements all evaluate cleanly
- **THEN** the result is a `Value::List` of the evaluated elements in order.

#### Scenario: an element error propagates immediately

- **WHEN** `eval` evaluates an `Expr::List` whose first element raises an
  `EvalError`
- **THEN** that error is returned and the remaining elements are not evaluated.
