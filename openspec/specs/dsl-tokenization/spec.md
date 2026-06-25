# DSL Tokenization

## Purpose

Define the closed lexical token vocabulary of the `.steer` DSL and the rules by
which characters are classified into tokens (identifiers, numbers, comments,
operator greedy matching, and `@`). This capability covers the surface token set
produced by the lexer and the per-character classification rules; it does not
cover statement/grammar structure, string internals, or newline suppression,
which live in sibling capabilities.

## Requirements

### Requirement: The lexer emits a fixed, closed token vocabulary

The lexer SHALL emit exactly the token variants of the re-exported, closed
`Token` enum: `Ident(String)`, `Int(i64)`, `Float(f64)`,
`String(Vec<Spanned<StringSegment>>)`, `LParen`, `RParen`, `LBracket`,
`RBracket`, `Comma`, `Dot`, `At`, `Assign`, `Equal`, `NotEq`, `Lt`, `Gt`,
`LtEq`, `GtEq`, `Plus`, `Minus`, `Star`, `Slash`, `Newline`, and `Eof`. The
token stream SHALL always terminate with exactly one `Eof`. Adding a variant to
`Token` is a breaking change to this DSL surface. Evidenced by the public
`tokenize` entry point (`tokenize(src: &str) -> Result<Vec<Spanned<Token>>,
LexError>`), `enum Token`, and the test `all_operators` in
`crates/steer-syntax/src/lexer.rs`.

#### Scenario: the token stream ends with exactly one Eof

- **WHEN** any source text is tokenized
- **THEN** the resulting token vector's final element is an `Eof` and no other
  `Eof` appears earlier in the vector.

#### Scenario: the full operator and punctuation set is recognized

- **WHEN** a source exercises every operator and punctuation token (`(`, `)`,
  `[`, `]`, `,`, `.`, `@`, `=`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `+`, `-`, `*`,
  `/`)
- **THEN** each is produced as its dedicated single token variant.

### Requirement: Control-flow words lex as ordinary identifiers

The lexer SHALL NOT treat control-flow or operator-keyword words as lexical
keywords. `if`, `else`, `elseif`, `end`, `loop`, `until`, `for`, `in`, `func`,
`return`, `not`, `and`, and `or` SHALL each lex as `Ident(String)` carrying the
word's text, and SHALL be distinguished from ordinary identifiers only by the
parser. Evidenced by the test `control_keywords_are_idents` in
`crates/steer-syntax/src/lexer.rs`.

#### Scenario: an if-condition lexes as identifiers and operators

- **WHEN** the source `if x > 3` is tokenized
- **THEN** the emitted tokens are `Ident("if")`, `Ident("x")`, `Gt`, `Int(3)`.

#### Scenario: every reserved word is an Ident

- **WHEN** each of `if`, `else`, `elseif`, `end`, `loop`, `until`, `for`, `in`,
  `func`, `return`, `not`, `and`, `or` is tokenized in isolation
- **THEN** each produces a single `Ident` token whose text is that word.

### Requirement: Identifiers follow an ASCII letter-or-underscore rule

An identifier SHALL start with an ASCII letter or `_` and continue with ASCII
alphanumeric or `_` (the pattern `[_A-Za-z][_0-9A-Za-z]*`). Non-ASCII letters
SHALL NOT be accepted as identifier starts; the lexer SHALL report
`LexErrorKind::UnexpectedChar(char)` for such a start. Evidenced by `lex_ident`
and the `!`/non-ASCII branches in `crates/steer-syntax/src/lexer.rs` and the test
`error_unexpected_char`.

#### Scenario: a typical identifier is recognized

- **WHEN** the source `foo_bar2` is tokenized
- **THEN** the lexer emits a single `Ident("foo_bar2")`.

#### Scenario: a non-ASCII letter start is rejected

- **WHEN** an identifier starts with a non-ASCII letter
- **THEN** tokenization fails with `LexErrorKind::UnexpectedChar` carrying that
  character.

### Requirement: Numeric literals are int or float by digit-dot rule

A numeric literal SHALL start with an ASCII digit. A `.` immediately followed by
a digit SHALL form a `Float` (`1.5`, `3.0`); a trailing `.` with no following
digit SHALL NOT form a float (`3` stays `Int` and the `.` becomes a separate
`Dot`). Numeric text SHALL parse as `i64` or `f64`, and text that fails to parse
SHALL report `LexErrorKind::InvalidNumber(String)`. Evidenced by `lex_number` in
`crates/steer-syntax/src/lexer.rs` and the test `float_literals`.

#### Scenario: a dotted number is a float

- **WHEN** the source `1.5` is tokenized
- **THEN** the lexer emits a single `Float(1.5)`.

#### Scenario: a trailing dot does not form a float

- **WHEN** the source `3.x` is tokenized
- **THEN** the lexer emits `Int(3)`, `Dot`, then `Ident("x")` rather than a
  single float.

#### Scenario: unparseable numeric text is rejected

- **WHEN** digit-prefixed text cannot be parsed as `i64` or `f64`
- **THEN** tokenization fails with `LexErrorKind::InvalidNumber`.

### Requirement: Comments and insignificant whitespace produce no tokens

A `//` SHALL start a line comment that consumes characters up to (but not
including) the next `\n` and produces zero tokens. Space, `\t`, and `\r` SHALL be
insignificant anywhere. Evidenced by the comment branch and whitespace handling
in `crates/steer-syntax/src/lexer.rs` and the test `line_comment_inline`.

#### Scenario: a trailing comment is dropped

- **WHEN** the source `x // note` is tokenized
- **THEN** only `Ident("x")` (plus the trailing terminator) is emitted; the
  `// note` text yields no token.

#### Scenario: spaces and tabs are insignificant

- **WHEN** source contains arbitrary runs of spaces and tabs between tokens
- **THEN** no token is emitted for those characters.

### Requirement: Multi-character operators are matched greedily

The lexer SHALL match the two-character operators `==`, `!=`, `<=`, and `>=`
each as a single token, and SHALL match `=` as `Assign` and a lone `!` as an
error (the only legal use of `!` is as part of `!=`). The single-character
operators `<`, `>`, `+`, `-`, `*`, `/` SHALL each be one token. A lone `!`
SHALL report `LexErrorKind::UnexpectedChar('!')`. Evidenced by the operator
branches in `lex_token` in `crates/steer-syntax/src/lexer.rs`.

#### Scenario: equality and comparison operators are single tokens

- **WHEN** the source `== != <= >=` is tokenized
- **THEN** the emitted tokens are `Equal`, `NotEq`, `LtEq`, `GtEq`.

#### Scenario: a lone bang is an error

- **WHEN** the source contains a `!` not immediately followed by `=`
- **THEN** tokenization fails with `LexErrorKind::UnexpectedChar('!')`.

### Requirement: `@` is a standalone token

The `@` character SHALL lex as the standalone `At` token (the marker for a meta
directive). Evidenced by the `@` branch in `lex_token` in
`crates/steer-syntax/src/lexer.rs` and the test `meta_directive_tokens`.

#### Scenario: an at-sign starts a meta directive

- **WHEN** the source `@template = "t"` is tokenized
- **THEN** the first emitted token is `At`, followed by `Ident("template")`.
