# DSL Newline Handling

## Purpose

Define the line-oriented statement-termination model of the `.steer` DSL: when a
`\n` becomes a `Newline` token and when it is suppressed as insignificant
whitespace so that calls and lists may span physical lines. This capability
covers the newline-suppression depth model only; the token vocabulary lives in
[dsl-tokenization](openspec/specs/dsl-tokenization/spec.md) and statement
grammar in [dsl-statement-grammar](openspec/specs/dsl-statement-grammar/spec.md).

## Requirements

### Requirement: A newline is a statement terminator only at top level

The lexer SHALL maintain a delimiter depth that is incremented on `(` or `[` and
saturated-decremented on `)` or `]`. A `\n` SHALL emit a `Newline` token only
when delimiter depth is zero AND the current line emitted at least one token;
blank and comment-only lines SHALL produce no `Newline` token. Evidenced by the
`delimiter_depth` / `has_token_on_line` state and the `\n` branch in
`crates/steer-syntax/src/lexer.rs`, and the test
`line_comment_only_and_blank_lines_produce_no_newline`.

#### Scenario: a normal line is terminated

- **WHEN** a top-level line containing tokens is followed by `\n`
- **THEN** a `Newline` token is emitted at depth zero.

#### Scenario: blank and comment-only lines produce no Newline

- **WHEN** a source line contains only whitespace or only a `//` comment
- **THEN** no `Newline` token is emitted for that line.

### Requirement: Newlines are suppressed inside parentheses and brackets

When delimiter depth is greater than zero, a `\n` SHALL emit no token, so that a
call or list may span multiple physical lines. A multi-line top-level call SHALL
produce exactly one trailing top-level `Newline`. Missing delimiters (depth
greater than zero at end of input) are a parser concern, not a lex concern.
Evidenced by the `\n` and paren/bracket branches in
`crates/steer-syntax/src/lexer.rs` and the tests
`multiline_call_suppresses_inner_newlines` and
`multiline_call_preserves_outer_newline_only`.

#### Scenario: inner newlines of a multiline call are suppressed

- **WHEN** a call spans several physical lines inside its parentheses
- **THEN** no `Newline` token is emitted for the inner `\n` characters.

#### Scenario: a multiline call keeps one outer terminator

- **WHEN** a top-level call spans multiple lines and ends with `\n` at depth
  zero
- **THEN** exactly one trailing `Newline` token is emitted for the statement.

### Requirement: A final unterminated line is synthesized as a Newline

The lexer SHALL synthesize a `Newline` at the source length when the last line
emitted at least one token, has no trailing `\n`, and delimiter depth is zero.
The lexer SHALL NOT synthesize a `Newline` when delimiter depth is greater than
zero at end of input (an unclosed delimiter); the parser SHALL report the missing
delimiter. Evidenced by the end-of-`tokenize` logic in
`crates/steer-syntax/src/lexer.rs` and the test
`no_trailing_newline_still_terminated`.

#### Scenario: a final line without a trailing newline is still terminated

- **WHEN** the last line of source has tokens but no trailing `\n` and all
  delimiters are balanced
- **THEN** a `Newline` is synthesized at the source length.

#### Scenario: an unclosed delimiter suppresses the synthesized newline

- **WHEN** the source ends with an unbalanced `(` or `[` (depth greater than zero
  at EOF)
- **THEN** no `Newline` is synthesized, and the parser reports the missing
  delimiter.

### Requirement: Braces are not tracked as delimiters

The lexer SHALL NOT track `{` or `}` for newline suppression. Braces appear only
inside string interpolation; they have no role in the line-termination model.
Note: `docs/specs/lexing.md` (around line 22) states that calls may span lines
"inside parentheses, brackets, or braces" — the code tracks only `(` and `[`, so
"or braces" is stale; the code is ground truth. Evidenced by the paren/bracket
branches (no brace branch) in `crates/steer-syntax/src/lexer.rs`.

#### Scenario: a brace does not suppress a newline

- **WHEN** a top-level line contains a `{` followed by a `\n`
- **THEN** the `\n` still emits a `Newline` token, because braces are not
  delimiter-depth-affecting.

### Requirement: Delimiter depth saturates and Eof has an empty span

A `)` or `]` SHALL saturate the delimiter depth to a minimum of zero (so
stray closers cannot drive it negative). The `Eof` token SHALL carry an empty
span at `source_len..source_len`. Evidenced by the saturating-decrement in the
paren/bracket branches and the `Eof` construction in
`crates/steer-syntax/src/lexer.rs`, and the test `eof_span_is_at_end`.

#### Scenario: a stray closer does not drive depth negative

- **WHEN** a `)` or `]` appears without a matching opener
- **THEN** delimiter depth saturates at zero rather than going negative.

#### Scenario: Eof has an empty span at source length

- **WHEN** any source is tokenized
- **THEN** the final `Eof` token's span is `source_len..source_len`.
