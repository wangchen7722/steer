# Parsing And AST

> Behavior specs for the parser and AST: statement forms, control structures, operator precedence, and reserved words.

## Scenario: public AST names match the implemented model
- **WHEN** consumers inspect the syntax AST
- **THEN** statements use `Stmt::Call` for standalone calls, expressions use
  `Expr::String`, string pieces use `StringPart::{Literal,Interpolation}`, call
  arguments use `CallArg::{Positional,Named}`, and binary operators use
  `BinaryOp`.

## Scenario: standalone non-call expressions are rejected
- **WHEN** a workflow contains a bare expression statement such as `1 + 2`,
  `"hello"`, or `x == y`
- **THEN** parsing fails because a standalone statement must be a call.

## Scenario: assignment and calls
- **WHEN** the source is `x = 5`, `task("do")`, or
  `result = task("do", return="path")`
- **THEN** parsing produces the corresponding `Assign` or `Call` AST.

## Scenario: meta directives
- **WHEN** the source contains `@template = "planning"`
- **THEN** parsing produces `Stmt::Meta { key: "template", value: ... }`.

## Scenario: control structures parse to their AST forms
- **WHEN** the source contains `if/elseif/else/end`, `loop ... until cond`,
  `for x in list ... end`, `func ... end`, or `return expr`
- **THEN** each parses to the corresponding statement with its body block.

## Scenario: operator precedence
- **WHEN** the source is `1 + 2 * 3`
- **THEN** parsing groups it as `1 + (2 * 3)`.

## Scenario: positional argument after a named one is rejected
- **WHEN** a call lists a positional argument after a named one
- **THEN** parsing returns `ParseErrorKind::PositionalAfterNamed`.

## Scenario: reserved words are not identifiers
- **WHEN** `not`, `and`, `or`, or `in` is used as an assignment target,
  argument name, or bare variable
- **THEN** parsing rejects it.
