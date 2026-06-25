# DSL String Literals

## Purpose

Define double-quoted string literals in the `.steer` DSL: the verbatim and
escape-segment rules, the fixed escape set, the `{...}` interpolation form, and
the single-physical-line constraint. A string literal is lexed into a single
`String` token carrying `Vec<Spanned<StringSegment>>`; the lexer does not parse
interpolation text, which is deferred to the parser. This capability covers only
string internals; the surrounding token vocabulary lives in
[dsl-tokenization](openspec/specs/dsl-tokenization/spec.md) and source spans in
[dsl-source-spans-and-errors](openspec/specs/dsl-source-spans-and-errors/spec.md).

## Requirements

### Requirement: A string literal is double-quoted and single-line

A string literal SHALL be delimited by `"` and SHALL span exactly one physical
line. A raw `\n` inside the quote SHALL fail. Each segment of the literal SHALL
carry a source span. Evidenced by `lex_string` in
`crates/steer-syntax/src/lexer.rs` and the tests `raw_newline_in_string_is_rejected`
and `error_unterminated_string`.

#### Scenario: a raw newline inside a string is rejected

- **WHEN** a `"`-delimited string contains a raw `\n`
- **THEN** tokenization fails with `NewlineInString`.

#### Scenario: an unterminated string is rejected

- **WHEN** a `"` opens a string that is never closed on its line
- **THEN** tokenization fails with `UnterminatedString`.

### Requirement: The escape set is fixed and closed

The lexer SHALL accept exactly these escape sequences and no others: `\n` (to
newline), `\t` (to tab), `\"` (to `"`), `\\` (to `\`), `\{` (to `{`), and `\}`
(to `}`). Any other `\x` SHALL fail with `LexErrorKind::InvalidEscape(char)`.
Evidenced by the escape branch in `lex_string` and the test `string_escapes`.

#### Scenario: every allowed escape is decoded

- **WHEN** a string contains `\n`, `\t`, `\"`, `\\`, `\{`, or `\}`
- **THEN** each is decoded to its corresponding literal character.

#### Scenario: an unknown escape is rejected

- **WHEN** a string contains an escape not in the fixed set (e.g. `\q`)
- **THEN** tokenization fails with `InvalidEscape` carrying the offending
  character.

### Requirement: Interpolation carries its inner text verbatim

A `{...}` region inside a string literal SHALL produce an
`Interpolation(String)` segment whose text is the inner expression source
verbatim (e.g. `{name}` yields the inner text `name`). The lexer SHALL NOT parse
the interpolation body; parsing is deferred to the parser, which relocates the
inner span to global source coordinates. Evidenced by the interpolation branch
of `lex_string`, `convert_str_segments` / `parse_interp_expr` in
`crates/steer-syntax/src/parser.rs`, and the test `string_interpolation`.

#### Scenario: an interpolation segment records its inner text

- **WHEN** the string `"hello {name}!"` is tokenized
- **THEN** the lexer emits segments `Literal("hello ")`,
  `Interpolation("name")`, `Literal("!")`, and the inner text is preserved
  verbatim for later parsing.

#### Scenario: interpolation spans are relocated to global coordinates

- **WHEN** the parser processes a string with an interpolation segment
- **THEN** the sub-expression's span is computed by offsetting from the segment's
  span (one past the opening brace), yielding global source coordinates.

### Requirement: Unicode is preserved inside string literals

The lexer SHALL preserve Unicode scalar values inside both literal and
interpolation segments of a string. Evidenced by `lex_string` in
`crates/steer-syntax/src/lexer.rs` and the test `unicode_in_strings`.

#### Scenario: non-ASCII text survives lexing

- **WHEN** a string literal contains non-ASCII characters
- **THEN** those characters appear verbatim in the resulting literal segment.

### Requirement: Interpolation bodies reject nested quotes and braces

Inside an interpolation body the lexer SHALL reject a `"` or an unescaped `{`
with `UnexpectedChar`, and an unterminated interpolation (no closing `}`) SHALL
fail with `UnterminatedInterpolation`. Evidenced by the interpolation branch of
`lex_string` in `crates/steer-syntax/src/lexer.rs` and the test
`interpolation_rejects_a_nested_string_or_brace`.

#### Scenario: a nested string inside interpolation is rejected

- **WHEN** an interpolation body contains a `"`
- **THEN** tokenization fails with `UnexpectedChar` carrying `"`.

#### Scenario: an unterminated interpolation is rejected

- **WHEN** an interpolation body is never closed by `}`
- **THEN** tokenization fails with `UnterminatedInterpolation`.

### Requirement: An interpolation body parses to a single expression

The parser SHALL accept an interpolation body only when it parses to exactly one
expression; an empty body or a body with trailing tokens after the first
expression SHALL be rejected with an end-of-interpolation error. This is distinct
from the lexer's rejection of nested quotes or braces and from unterminated
interpolations. Evidenced by `parse_interp_expr` in
`crates/steer-syntax/src/parser.rs`.

#### Scenario: a single-expression interpolation is accepted

- **WHEN** a string contains an interpolation whose body is a single expression
  such as `{name}`
- **THEN** the parser accepts it and binds the parsed expression.

#### Scenario: a multi-token interpolation body is rejected

- **WHEN** a string contains an interpolation body with trailing tokens after the
  first expression (e.g. `{a b}`)
- **THEN** parsing fails with an end-of-interpolation error.

### Requirement: The StringSegment variant set is a closed compatibility surface

The public `StringSegment` enum SHALL be the closed set `{ Literal(String),
Interpolation(String) }`, re-exported from the crate root and carried by
`Token::String`. Adding a variant SHALL be a breaking change to the DSL surface
that external tooling depends on. Evidenced by `enum StringSegment` in
`crates/steer-syntax/src/lexer.rs` and its re-export in
`crates/steer-syntax/src/lib.rs`.

#### Scenario: the segment kinds are literal and interpolation only

- **WHEN** a string literal is tokenized
- **THEN** every segment is either `Literal(String)` or `Interpolation(String)`.
