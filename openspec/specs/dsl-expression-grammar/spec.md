# DSL Expression Grammar

## Purpose

Define the expression grammar of the `.steer` DSL: the fixed operator-precedence
ladder, the call-argument grammar (positional then named), the primary forms,
and the set of word-operator keywords that are reserved and may not be used as
identifiers. This capability covers `Expr`/`Call`/`CallArg`/`BinaryOp`/`UnaryOp`
shapes; statement forms live in
[dsl-statement-grammar](openspec/specs/dsl-statement-grammar/spec.md) and the
token vocabulary in
[dsl-tokenization](openspec/specs/dsl-tokenization/spec.md).

## Requirements

### Requirement: Operators follow a fixed precedence ladder

Expressions SHALL parse with the following precedence, loosest to tightest: (1)
`or` (left-associative); (2) `and` (left-associative); (3) `not` (unary,
binding looser than comparison, so `not a == b` groups as `not (a == b)`); (4)
the comparison operators `==`, `!=`, `<`, `>`, `<=`, `>=` (non-associative, so
`a < b < c` is not a single chained comparison); (5) additive `+`/`-`; (6)
multiplicative `*`/`/`; (7) unary `-`; (8) primary. Evidenced by the precedence
ladder in `crates/steer-syntax/src/parser.rs` and the tests
`logical_precedence_or_binds_looser_than_and`,
`not_binds_looser_than_comparison`, and `precedence_add_mul`.

#### Scenario: or binds looser than and

- **WHEN** the expression `a or b and c` is parsed
- **THEN** it groups as `a or (b and c)`.

#### Scenario: not binds looser than comparison

- **WHEN** the expression `not a == b` is parsed
- **THEN** it groups as `not (a == b)`.

#### Scenario: comparison is non-associative

- **WHEN** the expression `a < b < c` is parsed
- **THEN** it is not accepted as a single chained comparison.

#### Scenario: additive binds looser than multiplicative

- **WHEN** the expression `1 + 2 * 3` is parsed
- **THEN** the multiplication groups tighter than the addition.

### Requirement: Comparison operators are non-associative

The comparison operators `==`, `!=`, `<`, `>`, `<=`, `>=` SHALL be
non-associative: a comparison SHALL NOT accept another comparison as its direct
operand on either side. Evidenced by the comparison level in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a chained comparison is rejected

- **WHEN** the expression `a < b < c` is parsed
- **THEN** parsing fails because comparison does not associate.

### Requirement: Primaries cover literals, lists, grouping, calls, and variables

The primary forms SHALL be: integer and float literals; string literals (with
interpolations parsed into `StringPart`); list literals `[a, b, c]`;
parenthesised grouping `( expr )` where the parentheses are not retained in the
AST; calls `Ident(...)`; and a bare variable `Ident`. Evidenced by `parse_primary`
in `crates/steer-syntax/src/parser.rs` and the test `list_literal`.

#### Scenario: a list literal is parsed

- **WHEN** the expression `[1, 2, 3]` is parsed
- **THEN** the result is a `List` whose elements are the parsed expressions.

#### Scenario: parentheses group without being retained

- **WHEN** the expression `(1 + 2)` is parsed
- **THEN** the result is the inner expression with no wrapper retaining the
  parentheses.

### Requirement: The public Expr and operator enum shapes are fixed

The re-exported `Expr` enum SHALL have exactly these variants: `Int(i64)`,
`Float(f64)`, `String(Vec<StringPart>)`, `List(Vec<Spanned<Expr>>)`,
`Var(String)`, `Call(Call)`, `Binary { op, lhs, rhs }`, and `Unary { op, expr }`.
`BinaryOp` SHALL be exactly `{Add, Sub, Mul, Div, Eq, Ne, Lt, Gt, Le, Ge, And,
Or}`; `UnaryOp` SHALL be exactly `{Neg, Not}`; `CallArg` SHALL be exactly
`{Positional, Named { name, value }}`; `StringPart` SHALL be exactly
`{Literal, Interpolation}`. Evidenced by the enums in
`crates/steer-syntax/src/ast.rs`; adding a variant is breaking.

#### Scenario: a binary expression uses the Binary variant

- **WHEN** the expression `1 + 2` is parsed
- **THEN** the result is `Expr::Binary` with `op == Add` and the parsed operands.

#### Scenario: a unary negation uses the Unary variant

- **WHEN** the expression `-x` is parsed
- **THEN** the result is `Expr::Unary` with `op == Neg`.

### Requirement: Call arguments are positional then named with no trailing comma

In a call, positional arguments SHALL precede any named arguments. A named
argument SHALL use the form `name = expr` (single `=`; `==` is the comparison
operator, not a named-argument binding). A positional argument appearing after
any named argument SHALL fail with `ParseErrorKind::PositionalAfterNamed` (span
at the offending positional). A trailing comma SHALL NOT be allowed in a call's
argument list or in a list literal. Evidenced by `parse_args` / `parse_arg` and
the tests `call_with_named_args` and `err_positional_after_named` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: named arguments are accepted after positional ones

- **WHEN** the call `task(a, name = "x")` is parsed
- **THEN** the result has one positional and one named argument.

#### Scenario: a positional after a named argument is rejected

- **WHEN** the call `task(name = "x", a)` is parsed
- **THEN** parsing fails with `PositionalAfterNamed` at the positional argument.

#### Scenario: a trailing comma is rejected

- **WHEN** a call or list literal ends with a trailing comma (e.g. `[1, 2,]`)
- **THEN** parsing fails.

### Requirement: Word-operator keywords are reserved and not usable as identifiers

The words `not`, `and`, `or`, and `in` SHALL be reserved by value in the parser.
Where they appear in a legal operator position they SHALL be consumed as
operators; elsewhere — as an assignment target, a named-argument name, or a bare
primary/variable — they SHALL be rejected with `UnexpectedToken`. Evidenced by
`is_operator_keyword` and its uses in `parse_primary`, `is_assign_start`, and
`parse_arg` in `crates/steer-syntax/src/parser.rs`, and the test
`operator_keywords_are_not_identifiers`.

#### Scenario: a reserved word as a value parses via its operator role

- **WHEN** the source `x = a or b` is parsed
- **THEN** it parses successfully because `or` is consumed as an operator.

#### Scenario: a reserved word as an assignment target is rejected

- **WHEN** the source `not = 5` is parsed
- **THEN** parsing fails because `not` is reserved.

#### Scenario: a reserved word as a named-argument name is rejected

- **WHEN** the call `task(not = 1)` is parsed
- **THEN** parsing fails because `not` is reserved.

#### Scenario: a reserved word as a trailing operand is rejected

- **WHEN** the source `x = a or` is parsed
- **THEN** parsing fails because `or` cannot be an operand.
