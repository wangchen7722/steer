# Lexing

> Behavior specs for the lexer: tokenization, string interpolation, comments, and span tracking.

## Scenario: tokenize an assignment
- **WHEN** the source is `x = 5`
- **THEN** the lexer emits `Ident("x"), Assign, Int(5), Newline, Eof`.

## Scenario: string with interpolation
- **WHEN** the source is `"hello {name}!"`
- **THEN** the lexer emits one `Token::String(Vec<Spanned<StringSegment>>)`
  token whose segments are `Literal("hello ")`, `Interpolation("name")`, and
  `Literal("!")`.

## Scenario: interpolation spans are source-global
- **WHEN** a string interpolation is parsed
- **THEN** the inner expression span maps back to the original source byte
  offsets, not to an interpolation-local `0..N` range.

## Scenario: line comments are ignored and multi-line calls are supported
- **WHEN** a line has a trailing `// comment`, or a call spans lines inside
  parentheses, brackets, or braces
- **THEN** comments produce no tokens and inner newlines are suppressed.

## Scenario: unclosed delimiters do not fabricate a top-level newline
- **WHEN** EOF is reached while delimiter depth is non-zero
- **THEN** the lexer does not emit a synthetic top-level `Newline`; the parser
  reports the missing delimiter.

## Scenario: raw newlines in strings are rejected
- **WHEN** a string literal crosses a physical line
- **THEN** lexing fails with `NewlineInString`; authors must use the `\n`
  escape.

## Scenario: interpolation bodies are restricted
- **WHEN** an interpolation body contains a raw `"` or nested `{`
- **THEN** lexing rejects it instead of silently mis-segmenting the string.

## Scenario: spans are byte offsets
- **WHEN** tokenizing `ab = 1`
- **THEN** `Ident("ab")` spans bytes `0..2` and `Int(1)` spans bytes `5..6`.
