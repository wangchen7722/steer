# DSL Statement Grammar

## Purpose

Define the statement layer of the `.steer` DSL grammar: the declarative
statement forms (meta, assignment, call-statement) and the block forms
(if/elseif/else/end, loop/until, for/in/end, func/end, return), as the
re-exported `Stmt` AST. This capability covers statement shapes and the
post-test loop semantics; expression and precedence rules live in
[dsl-expression-grammar](openspec/specs/dsl-expression-grammar/spec.md), token
vocabulary in [dsl-tokenization](openspec/specs/dsl-tokenization/spec.md), and
newline termination in
[dsl-newline-handling](openspec/specs/dsl-newline-handling/spec.md).

## Requirements

### Requirement: The module and block are the top-level AST shapes

The parser SHALL produce a `Module { body: Block }` where
`Block = Vec<Spanned<Stmt>>`, as the top-level AST shapes. `Module` and `Block`
shapes are a public AST surface. Evidenced by the public
`parse(src: &str) -> Result<Module, ParseError>` entry point, `Module` / `Block`
in `crates/steer-syntax/src/ast.rs`, and `parse` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a source parses to a module with a block body

- **WHEN** any valid source is parsed
- **THEN** the result is a `Module` whose `body` is a `Block` (vector of spanned
  statements).

### Requirement: Meta directives dot-join their key

A meta directive `@key.subkey = expr` SHALL parse into `Stmt::Meta { key, value }`
where `key` is the dot-joined identifier text (e.g. `@template.x = ...` yields
key `"template.x"`). Evidenced by `parse_meta` in
`crates/steer-syntax/src/parser.rs` and the test `meta_directive_tokens`.

#### Scenario: a dotted meta key is joined

- **WHEN** the source `@template.x = "t"` is parsed
- **THEN** the resulting `Meta` statement has `key == "template.x"`.

### Requirement: Assignments bind a target to a value expression

An assignment `target = expr` SHALL parse into `Stmt::Assign { target, value }`.
The target SHALL NOT be an operator keyword (`not`, `and`, `or`, `in`); see
[dsl-expression-grammar](openspec/specs/dsl-expression-grammar/spec.md).
Evidenced by `parse_stmt` / `is_assign_start` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a simple assignment is parsed

- **WHEN** the source `x = 1 + 2` is parsed
- **THEN** the result is an `Assign` whose target is `x` and whose value is the
  parsed expression.

### Requirement: A call statement is a bare call

A statement that begins with an identifier followed by `(` SHALL parse as
`Stmt::Call(Call)`. A bare non-call expression statement SHALL be REJECTED: a
standalone `1 + 2`, `"hello"`, `x == y`, or `[1, 2, 3]` as a statement SHALL fail
to parse. Evidenced by `parse_stmt` and the test
`bare_non_call_statement_is_rejected` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a call statement is accepted

- **WHEN** the source `task(name = "x")` is parsed as a statement
- **THEN** the result is `Stmt::Call` carrying the call.

#### Scenario: a bare expression statement is rejected

- **WHEN** a statement is a bare expression that is not a call (e.g. `1 + 2`)
- **THEN** parsing fails.

### Requirement: if/elseif/else/end preserves branch order

An `if` block SHALL parse into `Stmt::If { branches: Vec<IfBranch>, else_block }`
with `branches` kept in source order. `elseif` and `else` are optional; the
block SHALL be closed by `end`. Evidenced by `parse_if` / `parse_if_branch` and
the test `if_else_end` in `crates/steer-syntax/src/parser.rs`.

#### Scenario: branches are kept in order

- **WHEN** an `if` with multiple `elseif` branches and an `else` is parsed
- **THEN** `branches` appears in source order, followed by `else_block`.

#### Scenario: a missing end is rejected

- **WHEN** an `if` block is not closed by `end`
- **THEN** parsing fails reporting an expected `end`.

### Requirement: loop/until is the only loop form and is post-test

The ONLY loop form SHALL be `loop ... until cond`, parsed into
`Stmt::LoopUntil { body, cond }`. It SHALL be post-test: the body is parsed as a
block and the loop is expected to run the body at least once. There SHALL be no
counted loop and no `repeat` keyword. Evidenced by `parse_loop` and the test
`loop_until` in `crates/steer-syntax/src/parser.rs`.

#### Scenario: a loop until block is parsed

- **WHEN** the source `loop\n  step()\nuntil x > 3` is parsed
- **THEN** the result is a `LoopUntil` with the parsed body and condition.

### Requirement: for/in/end binds a variable to an iterable

A `for var in iterable <block> end` SHALL parse into
`Stmt::For { var, iterable, body }`. The loop variable SHALL be a bare
identifier. Evidenced by `parse_for` and the test `for_in` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a for-in block is parsed

- **WHEN** the source `for x in items\n  step()\nend` is parsed
- **THEN** the result is a `For` with `var == "x"`, the parsed iterable, and the
  parsed body.

### Requirement: func/end declares a named function with identifier params

A `func name(p1, p2) <block> end` SHALL parse into
`Stmt::Function { name, params, body }` where `params` is a comma-separated list
of bare identifiers. Evidenced by `parse_func` and the test `func_def` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a function definition is parsed

- **WHEN** the source `func add(a, b)\n  return a\nend` is parsed
- **THEN** the result is a `Function` with `name == "add"`, `params == ["a", "b"]`,
  and the parsed body.

### Requirement: return may carry a value or be bare

A `return [expr]` SHALL parse into `Stmt::Return { value }`. A bare `return`
with no expression SHALL be legal and yield `value == None`. Evidenced by
`parse_return` and the test `return_bare_and_value` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a bare return is legal

- **WHEN** the source `return` is parsed as a statement
- **THEN** the result is a `Return` with no value.

#### Scenario: a valued return carries its expression

- **WHEN** the source `return x + 1` is parsed as a statement
- **THEN** the result is a `Return` whose value is the parsed expression.

### Requirement: Block terminators delimit statements and stray terminators error

A statement SHALL be terminated by a newline, EOF, or a block-terminator token
(`end`, `else`, `elseif`, `until`). A stray block terminator at the top level
(where no block is open) SHALL be an error. Evidenced by `is_block_terminator`
and the test `err_stray_end_at_top_level` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a stray end at top level is rejected

- **WHEN** an `end` token appears at the top level with no open block
- **THEN** parsing fails.

#### Scenario: a block terminator closes the innermost statement

- **WHEN** a block-terminator token is encountered inside an open block
- **THEN** it terminates the current statement and is consumed by the enclosing
  block parser.

### Requirement: The statement AST enum variant sets are a fixed public surface

The public `Stmt`, `IfBranch`, `Call`, and `CallArg` enum variant sets SHALL be
fixed: they are re-exported from the crate root via `pub use ast::*`, and adding
a variant SHALL be a breaking change to the DSL AST surface, on the same footing
as the expression enum shapes in
[dsl-expression-grammar](openspec/specs/dsl-expression-grammar/spec.md).
Evidenced by `enum Stmt` and the `IfBranch`, `Call`, and `CallArg` types in
`crates/steer-syntax/src/ast.rs` and the re-export in
`crates/steer-syntax/src/lib.rs`.

#### Scenario: the statement AST uses only its fixed variant sets

- **WHEN** source is parsed into the statement AST
- **THEN** every statement is one of the fixed `Stmt` variants, and `IfBranch`,
  `Call`, and `CallArg` each use only their fixed variant sets.
