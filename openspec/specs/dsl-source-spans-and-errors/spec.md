# DSL Source Spans and Errors

## Purpose

Define the byte-offset span model attached to every lexical and AST node and the
unified lex/parse error model of the `.steer` DSL front-end. This capability
covers `Span`/`Spanned<T>`, the `line_col` human-diagnostics helper, and the
single-error (no-recovery) error categories; it depends on the token vocabulary
in [dsl-tokenization](openspec/specs/dsl-tokenization/spec.md) and the statement
and expression grammars in
[dsl-statement-grammar](openspec/specs/dsl-statement-grammar/spec.md) and
[dsl-expression-grammar](openspec/specs/dsl-expression-grammar/spec.md).

## Requirements

### Requirement: Spans are byte offsets carried by every node

The public `pub type Span = std::ops::Range<usize>` SHALL denote byte offsets
(not characters, not display width), half-open as `[start, end)`. Every token,
statement, expression, call argument, and string segment SHALL carry a span into
the source, and the invariant `start <= end` SHALL always hold. The public
`pub struct Spanned<T> { pub value: T, pub span: Span }` pairs each node with its
span. Evidenced by `source.rs` and span attachment across
`crates/steer-syntax/src/{lexer.rs,parser.rs}`, and the tests
`spans_are_byte_offsets` and `spans_are_populated`.

#### Scenario: spans are byte offsets, not character counts

- **WHEN** a source containing multi-byte characters is tokenized
- **THEN** each token's span reflects byte offsets into the UTF-8 source.

#### Scenario: every node carries a populated span

- **WHEN** a non-trivial source is parsed
- **THEN** each statement, expression, call argument, and string segment has a
  span with `start <= end`.

### Requirement: Interpolation sub-expression spans are relocated to global coordinates

Sub-expressions parsed from a string interpolation SHALL have their spans
relocated to global source coordinates by offsetting from the interpolation
segment's span (the inner parser coordinates plus one past the opening brace).
Evidenced by `convert_str_segments` / `parse_interp_expr` and `offset_expr` /
`offset_spans` in `crates/steer-syntax/src/parser.rs`.

#### Scenario: an interpolation sub-expression points into the original source

- **WHEN** a string with an interpolation is parsed
- **THEN** the interpolation's sub-expression span lands at the correct global
  byte range in the original source.

### Requirement: Errors always carry a span

Both `LexError` and `ParseError` SHALL carry a `span`. Evidenced by the
`LexError` and `ParseError` structs in
`crates/steer-syntax/src/{lexer.rs,parser.rs}`.

#### Scenario: a lex error reports a span

- **WHEN** the lexer rejects a character or string
- **THEN** the resulting `LexError` carries a span locating the problem in the
  source.

#### Scenario: a parse error reports a span

- **WHEN** the parser rejects a token sequence
- **THEN** the resulting `ParseError` carries a span locating the problem.

### Requirement: Boundary spans are empty and at the source end

The `Eof` token SHALL carry an empty span at `source_len..source_len`. A
synthesized trailing `Newline` (see
[dsl-newline-handling](openspec/specs/dsl-newline-handling/spec.md)) SHALL carry
span `source_len..source_len`. Evidenced by the `Eof` construction and the
end-of-`tokenize` logic in `crates/steer-syntax/src/lexer.rs` and the test
`eof_span_is_at_end`.

#### Scenario: Eof has an empty span at source length

- **WHEN** any source is tokenized
- **THEN** the `Eof` token's span is `source_len..source_len`.

### Requirement: line_col maps a byte offset to 1-based line and column for humans

The public `pub fn line_col(src: &str, byte: usize) -> (usize, usize)` SHALL
return a 1-based `(line, col)` where the column counts Unicode scalar values.
It is intended for human-readable CLI diagnostics and SHALL NOT be treated as an
LSP protocol position. Evidenced by `line_col` in
`crates/steer-syntax/src/lexer.rs`.

#### Scenario: line_col returns 1-based coordinates

- **WHEN** `line_col` is called with a byte offset inside a source
- **THEN** it returns a 1-based `(line, col)` pair, with the column counting
  Unicode scalar values.

### Requirement: Parsing is a single-error model with no recovery

Parsing SHALL stop at the first unrecoverable error and SHALL NOT attempt
recovery. The public `pub fn parse(src) -> Result<Module, ParseError>` SHALL wrap
any lex error as `ParseErrorKind::Lex(LexError)` with its span preserved.
Grammar errors SHALL be `UnexpectedToken { expected, found }`,
`UnexpectedEof { expected }`, or `PositionalAfterNamed`. The `expected` field is
a human-readable string, not a structured classifier; the only machine-checkable
classifiers are the `ParseErrorKind` variant and the inner `LexErrorKind`.
Evidenced by `ParseErrorKind` / `ParseError` and the lex-wrap path in
`crates/steer-syntax/src/parser.rs`, and the tests `err_lex_error_propagates`,
`err_missing_end`, `err_unexpected_token`, and `err_positional_after_named`.

#### Scenario: a lex error propagates through parse

- **WHEN** `parse` is given source that fails to lex
- **THEN** it returns a `ParseError` whose kind is `Lex(...)` carrying the
  original `LexError` with its span preserved.

#### Scenario: parsing stops at the first error

- **WHEN** source contains more than one grammar error
- **THEN** `parse` returns exactly one `ParseError` for the first unrecoverable
  error and performs no recovery.

### Requirement: The error-kind enum shapes are a public compatibility surface

`LexErrorKind`, `LexError`, `ParseErrorKind`, and `ParseError` SHALL implement
`Display` and `Error` and SHALL be re-exported at the crate root. Adding a
variant to `LexErrorKind` or `ParseErrorKind` is a breaking change. Evidenced by
the enum and impl blocks in `crates/steer-syntax/src/{lexer.rs,parser.rs}` and
the crate-root re-exports in `crates/steer-syntax/src/lib.rs`.

#### Scenario: errors are displayable and re-exported

- **WHEN** a caller imports from the crate root and formats an error
- **THEN** the error implements `Display` + `Error` and is usable without
  reaching into a private module.
