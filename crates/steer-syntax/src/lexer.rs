//! Lexer for the steer workflow DSL.
//!
//! The lexer is line-oriented: each statement occupies one logical line, and a
//! newline at the top level terminates a statement. Newlines inside parentheses
//! or brackets are insignificant whitespace, which lets a long call such as
//! `task(...)` span multiple physical lines.
//!
//! Keywords (`if`, `end`, `loop`, ...) are not distinguished from identifiers
//! here; they are recognised by value in the parser. Spans are byte offsets
//! into the source so they can feed diagnostics and a future language server.

use thiserror::Error;

use crate::source::{Span, Spanned};

/// A segment of a string literal: a literal run of text or an interpolation
/// `{ ... }` whose inner expression text is captured verbatim.
#[derive(Debug, Clone, PartialEq)]
pub enum StringSegment {
    Literal(String),
    Interpolation(String),
}

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// An identifier or keyword; keywords are distinguished by the parser.
    Ident(String),
    /// An integer literal.
    Int(i64),
    /// A floating-point literal.
    Float(f64),
    /// A string literal, possibly with `{var}` interpolations. Each segment
    /// carries its source span; an interpolation span covers the whole `{...}`.
    String(Vec<Spanned<StringSegment>>),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    /// `@` — starts a workflow-level directive (`@template=`).
    At,
    // assignment / comparison / arithmetic operators
    Assign,
    Equal,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    /// End of a logical statement line, emitted at the top level only.
    Newline,
    /// End of input.
    Eof,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "identifier `{s}`"),
            Token::Int(n) => write!(f, "integer {n}"),
            Token::Float(x) => write!(f, "float {x}"),
            Token::String(_) => write!(f, "string literal"),
            Token::LParen => write!(f, "`(`"),
            Token::RParen => write!(f, "`)`"),
            Token::LBracket => write!(f, "`[`"),
            Token::RBracket => write!(f, "`]`"),
            Token::Comma => write!(f, "`,`"),
            Token::Dot => write!(f, "`.`"),
            Token::At => write!(f, "`@`"),
            Token::Assign => write!(f, "`=`"),
            Token::Equal => write!(f, "`==`"),
            Token::NotEq => write!(f, "`!=`"),
            Token::Lt => write!(f, "`<`"),
            Token::Gt => write!(f, "`>`"),
            Token::LtEq => write!(f, "`<=`"),
            Token::GtEq => write!(f, "`>=`"),
            Token::Plus => write!(f, "`+`"),
            Token::Minus => write!(f, "`-`"),
            Token::Star => write!(f, "`*`"),
            Token::Slash => write!(f, "`/`"),
            Token::Newline => write!(f, "end of line"),
            Token::Eof => write!(f, "end of input"),
        }
    }
}

/// Classification of a lexical error.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum LexErrorKind {
    #[error("unterminated string literal")]
    UnterminatedString,
    #[error("unescaped newline in string literal")]
    NewlineInString,
    #[error("unterminated interpolation `{{...}}` inside string")]
    UnterminatedInterpolation,
    #[error("invalid escape sequence `\\{0}`")]
    InvalidEscape(char),
    #[error("invalid number literal `{0}`")]
    InvalidNumber(String),
    #[error("unexpected character `{0}`")]
    UnexpectedChar(char),
}

/// A lexical error together with its source span.
#[derive(Debug, PartialEq, Clone)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at byte {})", self.kind, self.span.start)
    }
}

impl std::error::Error for LexError {}

/// The lexer. Tokenises a source string into a flat [`Token`] stream.
///
/// Deliberately not `pub`: `tokenize` is the public entry point. The struct is
/// an implementation detail shared between `tokenize` and the tests.
struct Lexer {
    /// `(byte_offset, char)` for every character in the source.
    chars: Vec<(usize, char)>,
    /// Current index into [`Self::chars`].
    i: usize,
    source_len: usize,
    /// Delimiter nesting depth (`(` `[`); newlines are suppressed while > 0.
    delimiter_depth: usize,
    /// Whether any token has been emitted on the current logical line.
    has_token_on_line: bool,
}

impl Lexer {
    /// Create a lexer over `src`.
    fn new(src: &str) -> Self {
        Lexer {
            chars: src.char_indices().collect(),
            i: 0,
            source_len: src.len(),
            delimiter_depth: 0,
            has_token_on_line: false,
        }
    }

    /// Current byte offset: start of the next unconsumed char, or end of input.
    fn pos(&self) -> usize {
        self.chars
            .get(self.i)
            .map(|(p, _)| *p)
            .unwrap_or(self.source_len)
    }

    fn peek(&self) -> Option<(usize, char)> {
        self.chars.get(self.i).copied()
    }

    fn peek_ch(&self) -> Option<char> {
        self.chars.get(self.i).map(|(_, c)| *c)
    }

    fn peek_second_ch(&self) -> Option<char> {
        self.chars.get(self.i + 1).map(|(_, c)| *c)
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let c = self.chars.get(self.i).copied();
        if c.is_some() {
            self.i += 1;
        }
        c
    }

    fn simple(&mut self, start: usize, value: Token) -> Spanned<Token> {
        Spanned {
            value,
            span: start..self.pos(),
        }
    }

    fn err(&self, start: usize, kind: LexErrorKind) -> LexError {
        LexError {
            kind,
            span: start..self.pos().max(start + 1),
        }
    }

    /// Produce the next token, or `None` at end of input.
    fn next_token(&mut self) -> Result<Option<Spanned<Token>>, LexError> {
        loop {
            let Some((pos, ch)) = self.peek() else {
                return Ok(None);
            };
            match ch {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    if self.delimiter_depth == 0 && self.has_token_on_line {
                        self.has_token_on_line = false;
                        return Ok(Some(Spanned {
                            value: Token::Newline,
                            span: pos..pos + 1,
                        }));
                    }
                    // inside parens, or a blank line: ignore the newline
                }
                '/' if self.peek_second_ch() == Some('/') => {
                    // line comment: consume up to, but not including, end of line
                    while let Some((_, c)) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => {
                    self.has_token_on_line = true;
                    return self.lex_token(pos, ch).map(Some);
                }
            }
        }
    }

    fn lex_token(&mut self, start: usize, first: char) -> Result<Spanned<Token>, LexError> {
        match first {
            '(' => {
                self.bump();
                self.delimiter_depth += 1;
                Ok(self.simple(start, Token::LParen))
            }
            ')' => {
                self.bump();
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                Ok(self.simple(start, Token::RParen))
            }
            '[' => {
                self.bump();
                self.delimiter_depth += 1;
                Ok(self.simple(start, Token::LBracket))
            }
            ']' => {
                self.bump();
                self.delimiter_depth = self.delimiter_depth.saturating_sub(1);
                Ok(self.simple(start, Token::RBracket))
            }
            ',' => {
                self.bump();
                Ok(self.simple(start, Token::Comma))
            }
            '.' => {
                self.bump();
                Ok(self.simple(start, Token::Dot))
            }
            '@' => {
                self.bump();
                Ok(self.simple(start, Token::At))
            }
            '=' => {
                self.bump();
                if self.peek_ch() == Some('=') {
                    self.bump();
                    Ok(self.simple(start, Token::Equal))
                } else {
                    Ok(self.simple(start, Token::Assign))
                }
            }
            '!' => {
                self.bump();
                if self.peek_ch() == Some('=') {
                    self.bump();
                    Ok(self.simple(start, Token::NotEq))
                } else {
                    Err(self.err(start, LexErrorKind::UnexpectedChar('!')))
                }
            }
            '<' => {
                self.bump();
                if self.peek_ch() == Some('=') {
                    self.bump();
                    Ok(self.simple(start, Token::LtEq))
                } else {
                    Ok(self.simple(start, Token::Lt))
                }
            }
            '>' => {
                self.bump();
                if self.peek_ch() == Some('=') {
                    self.bump();
                    Ok(self.simple(start, Token::GtEq))
                } else {
                    Ok(self.simple(start, Token::Gt))
                }
            }
            '+' => {
                self.bump();
                Ok(self.simple(start, Token::Plus))
            }
            '-' => {
                self.bump();
                Ok(self.simple(start, Token::Minus))
            }
            '*' => {
                self.bump();
                Ok(self.simple(start, Token::Star))
            }
            '/' => {
                self.bump();
                Ok(self.simple(start, Token::Slash))
            }
            '"' => self.lex_string(start),
            '0'..='9' => self.lex_number(start),
            c if c.is_ascii_alphabetic() || c == '_' => Ok(self.lex_ident(start)),
            _ => {
                self.bump();
                Err(self.err(start, LexErrorKind::UnexpectedChar(first)))
            }
        }
    }

    fn lex_ident(&mut self, start: usize) -> Spanned<Token> {
        // The first char (an ASCII letter or `_`) is still unconsumed in the
        // stream; consume it as part of the loop rather than pushing it twice.
        let mut s = String::new();
        while let Some(c) = self.peek_ch() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        self.simple(start, Token::Ident(s))
    }

    fn lex_number(&mut self, start: usize) -> Result<Spanned<Token>, LexError> {
        let mut s = String::new();
        let mut is_float = false;
        while let Some(c) = self.peek_ch() {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else if c == '.'
                && !is_float
                && self.peek_second_ch().is_some_and(|d| d.is_ascii_digit())
            {
                is_float = true;
                s.push('.');
                self.bump();
            } else {
                break;
            }
        }
        let tok = if is_float {
            s.parse::<f64>()
                .map(Token::Float)
                .map_err(|_| self.err(start, LexErrorKind::InvalidNumber(s.clone())))?
        } else {
            s.parse::<i64>()
                .map(Token::Int)
                .map_err(|_| self.err(start, LexErrorKind::InvalidNumber(s.clone())))?
        };
        Ok(self.simple(start, tok))
    }

    fn lex_string(&mut self, start: usize) -> Result<Spanned<Token>, LexError> {
        self.bump(); // consume opening quote
        let mut segs: Vec<Spanned<StringSegment>> = Vec::new();
        let mut lit = String::new();
        let mut lit_start = self.pos();
        let flush_literal =
            |segs: &mut Vec<_>, lit: &mut String, lit_start: &mut usize, end: usize| {
                if !lit.is_empty() {
                    segs.push(Spanned {
                        value: StringSegment::Literal(std::mem::take(lit)),
                        span: *lit_start..end,
                    });
                    *lit_start = end;
                }
            };
        loop {
            let Some((_, c)) = self.peek() else {
                return Err(LexError {
                    kind: LexErrorKind::UnterminatedString,
                    span: start..self.source_len,
                });
            };
            match c {
                '"' => {
                    self.bump();
                    break;
                }
                '\n' => {
                    // Strings occupy one physical line; use the `\n` escape.
                    let p = self.pos();
                    return Err(LexError {
                        kind: LexErrorKind::NewlineInString,
                        span: p..p + 1,
                    });
                }
                '\\' => {
                    self.bump(); // consume backslash
                    let Some((ep, esc)) = self.peek() else {
                        return Err(LexError {
                            kind: LexErrorKind::UnterminatedString,
                            span: start..self.source_len,
                        });
                    };
                    let mapped = match esc {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        '{' => '{',
                        '}' => '}',
                        _ => {
                            self.bump();
                            return Err(LexError {
                                kind: LexErrorKind::InvalidEscape(esc),
                                span: ep..ep + esc.len_utf8(),
                            });
                        }
                    };
                    lit.push(mapped);
                    self.bump(); // consume the escaped char
                }
                '{' => {
                    flush_literal(&mut segs, &mut lit, &mut lit_start, self.pos());
                    let brace_start = self.pos();
                    self.bump(); // consume '{'
                    let mut interp = String::new();
                    loop {
                        match self.peek_ch() {
                            None => {
                                return Err(LexError {
                                    kind: LexErrorKind::UnterminatedInterpolation,
                                    span: brace_start..self.source_len,
                                });
                            }
                            Some('}') => {
                                self.bump();
                                break;
                            }
                            // Interpolation bodies are restricted to simple
                            // expressions: a string literal or nested `{}` would
                            // need a delimiter-aware scanner, so reject them.
                            Some(ch @ ('"' | '{')) => {
                                let p = self.pos();
                                return Err(LexError {
                                    kind: LexErrorKind::UnexpectedChar(ch),
                                    span: p..p + 1,
                                });
                            }
                            Some(ch) => {
                                interp.push(ch);
                                self.bump();
                            }
                        }
                    }
                    // The segment span covers the whole `{...}`; the inner
                    // expression source range is `start+1..end-1`.
                    let brace_end = self.pos();
                    segs.push(Spanned {
                        value: StringSegment::Interpolation(interp),
                        span: brace_start..brace_end,
                    });
                    lit_start = brace_end;
                }
                _ => {
                    lit.push(c);
                    self.bump();
                }
            }
        }
        flush_literal(&mut segs, &mut lit, &mut lit_start, self.pos());
        Ok(self.simple(start, Token::String(segs)))
    }
}

/// Tokenise `src` into a token stream ending in [`Token::Eof`].
///
/// # Errors
/// Returns a [`LexError`] if the source contains an invalid token, e.g. an
/// unterminated string or an unexpected character.
pub fn tokenize(src: &str) -> Result<Vec<Spanned<Token>>, LexError> {
    let mut lx = Lexer::new(src);
    let mut out = Vec::new();
    while let Some(t) = lx.next_token()? {
        out.push(t);
    }
    // Flush a trailing newline if the final line had tokens but no newline. Do
    // not fabricate one while still inside an unclosed `(`/`[` group; let the
    // parser report the missing delimiter.
    if lx.delimiter_depth == 0 && lx.has_token_on_line {
        out.push(Spanned {
            value: Token::Newline,
            span: lx.source_len..lx.source_len,
        });
    }
    let end = lx.source_len;
    out.push(Spanned {
        value: Token::Eof,
        span: end..end,
    });
    Ok(out)
}

/// Convert a byte offset into a 1-based `(line, column)` pair within `src`.
///
/// The column counts Unicode scalar values, not bytes and not terminal
/// display width. It is meant for human-readable CLI diagnostics; it is not a
/// language-server position, which must use the protocol's selected
/// character encoding.
pub fn line_col(src: &str, byte: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Token kinds with spans dropped, for concise assertions. String-segment
    /// spans are normalized too so a `strlit(...)` helper can compare cleanly.
    fn kinds(src: &str) -> Vec<Token> {
        tokenize(src)
            .unwrap()
            .into_iter()
            .map(|s| drop_spans(s.value))
            .collect()
    }

    fn drop_spans(t: Token) -> Token {
        match t {
            Token::String(segs) => Token::String(
                segs.into_iter()
                    .map(|s| Spanned {
                        value: s.value,
                        span: 0..0,
                    })
                    .collect(),
            ),
            other => other,
        }
    }

    fn id(s: &str) -> Token {
        Token::Ident(s.to_string())
    }
    fn strlit(parts: &[&str]) -> Token {
        // convenience: a string of literal-only segments (spans irrelevant here).
        Token::String(
            parts
                .iter()
                .map(|p| Spanned {
                    value: StringSegment::Literal((*p).to_string()),
                    span: 0..0,
                })
                .collect(),
        )
    }

    #[test]
    fn assignment_int() {
        assert_eq!(
            kinds("x = 5"),
            vec![
                id("x"),
                Token::Assign,
                Token::Int(5),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn assignment_string() {
        assert_eq!(
            kinds("name = \"hello\""),
            vec![
                id("name"),
                Token::Assign,
                strlit(&["hello"]),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn string_interpolation() {
        let toks = tokenize("\"a {x} b\"").unwrap();
        // no trailing newline -> String, flushed Newline, Eof
        assert_eq!(toks.len(), 3);
        let Token::String(segs) = &toks[0].value else {
            panic!("expected string")
        };
        let values: Vec<_> = segs.iter().map(|s| s.value.clone()).collect();
        assert_eq!(
            values,
            vec![
                StringSegment::Literal("a ".into()),
                StringSegment::Interpolation("x".into()),
                StringSegment::Literal(" b".into()),
            ]
        );
        // The interpolation segment spans the whole `{x}`.
        assert_eq!(segs[1].span, 3..6);
    }

    #[test]
    fn unterminated_paren_does_not_emit_top_level_newline() {
        // `task(` is inside a delimiter group: EOF must not fabricate a Newline.
        let toks: Vec<Token> = tokenize("task(")
            .unwrap()
            .into_iter()
            .map(|s| s.value)
            .collect();
        assert_eq!(
            toks,
            vec![Token::Ident("task".into()), Token::LParen, Token::Eof]
        );
    }

    #[test]
    fn raw_newline_in_string_is_rejected() {
        assert!(tokenize("\"line one\nline two\"").is_err());
    }

    #[test]
    fn interpolation_rejects_a_nested_string_or_brace() {
        // Decision A: interpolation bodies hold simple expressions only.
        assert!(tokenize("\"{a \\\"}\\\"}\"").is_err());
        assert!(tokenize("\"{a {b}}\"").is_err());
    }

    #[test]
    fn string_escapes() {
        // escaped quote and brace, newline, tab
        let toks = kinds("\"a\\\"b\\{c\\n\\t\"");
        let Token::String(segs) = &toks[0] else {
            panic!()
        };
        let values: Vec<_> = segs.iter().map(|s| s.value.clone()).collect();
        assert_eq!(values, vec![StringSegment::Literal("a\"b{c\n\t".into())]);
    }

    #[test]
    fn line_comment_inline() {
        assert_eq!(
            kinds("x = 5 // set x"),
            vec![
                id("x"),
                Token::Assign,
                Token::Int(5),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn line_comment_only_and_blank_lines_produce_no_newline() {
        // comment-only and blank lines do not introduce extra newlines
        assert_eq!(
            kinds("// header\n\nx = 1\n\n\ny = 2\n"),
            vec![
                id("x"),
                Token::Assign,
                Token::Int(1),
                Token::Newline,
                id("y"),
                Token::Assign,
                Token::Int(2),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn no_trailing_newline_still_terminated() {
        assert_eq!(
            kinds("x = 5"),
            vec![
                id("x"),
                Token::Assign,
                Token::Int(5),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn multiline_call_suppresses_inner_newlines() {
        let src = "task(\"a\",\n     return=\"b\")\n";
        assert_eq!(
            kinds(src),
            vec![
                id("task"),
                Token::LParen,
                strlit(&["a"]),
                Token::Comma,
                id("return"),
                Token::Assign,
                strlit(&["b"]),
                Token::RParen,
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn all_operators() {
        assert_eq!(
            kinds("a = = == != < > <= >= + - * /"),
            vec![
                id("a"),
                Token::Assign,
                Token::Assign,
                Token::Equal,
                Token::NotEq,
                Token::Lt,
                Token::Gt,
                Token::LtEq,
                Token::GtEq,
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn float_literals() {
        assert_eq!(kinds("x = 1.5")[2], Token::Float(1.5));
        assert_eq!(kinds("x = 3.0")[2], Token::Float(3.0));
        // a trailing dot without digits is NOT a float
        let toks = kinds("x = 3");
        assert_eq!(toks[2], Token::Int(3));
    }

    #[test]
    fn control_keywords_are_idents() {
        // keywords are lexed as identifiers; the parser interprets them
        assert_eq!(
            kinds("if x > 3\nelse\nend\n"),
            vec![
                id("if"),
                id("x"),
                Token::Gt,
                Token::Int(3),
                Token::Newline,
                id("else"),
                Token::Newline,
                id("end"),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn loops_and_func() {
        assert_eq!(
            kinds("loop\nfor item in list\nfunc add(a, b)\n"),
            vec![
                id("loop"),
                Token::Newline,
                id("for"),
                id("item"),
                id("in"),
                id("list"),
                Token::Newline,
                id("func"),
                id("add"),
                Token::LParen,
                id("a"),
                Token::Comma,
                id("b"),
                Token::RParen,
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn produce_list_literal() {
        assert_eq!(
            kinds("produce=[\"a\", \"b\"]"),
            vec![
                id("produce"),
                Token::Assign,
                Token::LBracket,
                strlit(&["a"]),
                Token::Comma,
                strlit(&["b"]),
                Token::RBracket,
                Token::Newline,
                Token::Eof
            ]
        );
    }

    #[test]
    fn unicode_in_strings() {
        // non-ASCII content inside strings is preserved
        let toks = kinds("\"你好 {name}\"");
        let Token::String(segs) = &toks[0] else {
            panic!()
        };
        let values: Vec<_> = segs.iter().map(|s| s.value.clone()).collect();
        assert_eq!(
            values,
            vec![
                StringSegment::Literal("你好 ".into()),
                StringSegment::Interpolation("name".into()),
            ]
        );
    }

    #[test]
    fn error_unterminated_string() {
        let err = tokenize("\"abc").unwrap_err();
        assert_eq!(err.kind, LexErrorKind::UnterminatedString);
    }

    #[test]
    fn error_unterminated_interp() {
        let err = tokenize("\"abc {x").unwrap_err();
        assert_eq!(err.kind, LexErrorKind::UnterminatedInterpolation);
    }

    #[test]
    fn error_invalid_escape() {
        let err = tokenize("\"a\\q\"").unwrap_err();
        assert_eq!(err.kind, LexErrorKind::InvalidEscape('q'));
    }

    #[test]
    fn error_unexpected_char() {
        let err = tokenize("x = $").unwrap_err();
        assert_eq!(err.kind, LexErrorKind::UnexpectedChar('$'));
    }

    #[test]
    fn meta_directive_tokens() {
        assert_eq!(
            kinds("@template = \"bugfix\""),
            vec![
                Token::At,
                Token::Ident("template".into()),
                Token::Assign,
                strlit(&["bugfix"]),
                Token::Newline,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn spans_are_byte_offsets() {
        let toks = tokenize("ab = 1").unwrap();
        // "ab" spans bytes 0..2
        assert_eq!(toks[0].span, 0..2);
        // "1" spans bytes 5..6
        assert_eq!(toks[2].span, 5..6);
    }

    #[test]
    fn eof_span_is_at_end() {
        let toks = tokenize("x = 1\n").unwrap();
        let eof = toks.last().unwrap();
        assert_eq!(eof.value, Token::Eof);
        assert_eq!(eof.span, 6..6);
    }

    #[test]
    fn multiline_call_preserves_outer_newline_only() {
        // two multiline calls; each yields exactly one Newline at the top level
        let src = "task(\"a\",\n return=\"b\")\ntask(\"c\",\n return=\"d\")\n";
        let toks = kinds(src);
        let newlines = toks.iter().filter(|t| **t == Token::Newline).count();
        assert_eq!(newlines, 2);
    }
}
