//! Recursive-descent parser: token stream -> AST.
//!
//! Grammar (informal):
//!
//! ```text
//! module      := stmt*
//! stmt        := meta | assign | exprStmt | if | loop | for | func | return
//! meta        := "@" IDENT ("." IDENT)* "=" expr
//! assign      := IDENT "=" expr
//! exprStmt    := expr
//! if          := "if" expr sep block ("elseif" expr sep block)* ("else" sep block)? "end" sep
//! loop        := "loop" sep block "until" expr sep
//! for         := "for" IDENT "in" expr sep block "end" sep
//! func        := "func" IDENT "(" params? ")" sep block "end" sep
//! return      := "return" expr? sep
//! expr        := or
//! or          := and ("or" and)*
//! and         := not ("and" not)*
//! not         := "not" not | comparison
//! comparison  := additive ( ("=="|"!="|"<"|">"|"<="|">=") additive )?
//! additive    := mul ( ("+"|"-") mul )*
//! mul         := unary ( ("*"|"/") unary )*
//! unary       := "-" unary | primary
//! primary     := INT | FLOAT | STR | list | "(" expr ")" | call | var
//! list        := "[" (expr ("," expr)*)? "]"
//! call        := IDENT "(" (arg ("," arg)*)? ")"
//! arg         := IDENT "=" expr     (named)
//!              | expr               (positional; must precede any named)
//! ```
//!
//! `sep` is a statement separator: a newline, or the end of the input, or a
//! block terminator (`end` / `else` / `until`) which is left for the caller.

use crate::ast::*;
use crate::lexer::{tokenize, LexError, Token};
use crate::source::{Span, Spanned};

/// Classification of a parse error.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// A lexical error occurred while tokenising the source.
    Lex(LexError),
    /// The parser expected `expected` but found `found`.
    UnexpectedToken { expected: String, found: Token },
    /// The parser ran out of tokens while expecting `expected`.
    UnexpectedEof { expected: String },
    /// A positional argument appeared after a named argument.
    PositionalAfterNamed,
}

/// A parse error together with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ParseErrorKind::Lex(e) => write!(f, "{e}"),
            ParseErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            ParseErrorKind::UnexpectedEof { expected } => {
                write!(f, "expected {expected}, found end of input")
            }
            ParseErrorKind::PositionalAfterNamed => {
                write!(f, "positional argument cannot follow a named argument")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a source string into a [`Module`].
///
/// # Errors
/// Returns a [`ParseError`] on any lexical or syntactic error.
pub fn parse(src: &str) -> Result<Module, ParseError> {
    let toks = tokenize(src).map_err(|e| ParseError {
        kind: ParseErrorKind::Lex(e.clone()),
        span: e.span,
    })?;
    let mut p = Parser::new(toks);
    let body = p.parse_block()?;
    // At the top level a stray block terminator is an error.
    if let Some(t) = p.peek() {
        if is_terminator(&t.value) {
            return Err(Parser::unexpected_tok("end of workflow", t.clone()));
        }
    }
    p.expect_eof()?;
    Ok(Module { body })
}

/// Recursive-descent parser over a token stream.
pub struct Parser {
    toks: Vec<Spanned<Token>>,
    i: usize,
    /// Byte offset where the last consumed token ended.
    prev_end: usize,
}

impl Parser {
    pub fn new(toks: Vec<Spanned<Token>>) -> Self {
        let prev_end = toks.last().map(|t| t.span.end).unwrap_or(0);
        Parser {
            toks,
            i: 0,
            prev_end,
        }
    }

    // ---- low-level cursor helpers ----

    fn peek(&self) -> Option<&Spanned<Token>> {
        self.toks.get(self.i)
    }

    fn peek_tok(&self) -> &Token {
        static EOF: Token = Token::Eof;
        self.toks.get(self.i).map(|s| &s.value).unwrap_or(&EOF)
    }

    fn bump(&mut self) -> Option<Spanned<Token>> {
        let t = self.toks.get(self.i).cloned();
        if t.is_some() {
            self.prev_end = self.toks[self.i].span.end;
            self.i += 1;
        }
        t
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_tok(), Token::Eof)
    }

    fn cur_start(&self) -> usize {
        self.toks
            .get(self.i)
            .map(|t| t.span.start)
            .unwrap_or(self.prev_end)
    }

    fn spanned_from<T>(&self, start: usize, value: T) -> Spanned<T> {
        Spanned {
            value,
            span: start..self.prev_end.max(start),
        }
    }

    fn peek_ident_value(&self) -> Option<&str> {
        if let Some(Spanned {
            value: Token::Ident(s),
            ..
        }) = self.peek()
        {
            Some(s)
        } else {
            None
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek_tok(), Token::Newline) {
            self.bump();
        }
    }

    fn is_block_terminator(&self) -> bool {
        matches!(
            self.peek_ident_value(),
            Some("end" | "else" | "elseif" | "until")
        )
    }

    // ---- error helpers ----

    fn unexpected(&self, expected: &str) -> ParseError {
        match self.peek() {
            Some(t) => Self::unexpected_tok(expected, t.clone()),
            None => ParseError {
                kind: ParseErrorKind::UnexpectedEof {
                    expected: expected.to_string(),
                },
                span: self.prev_end..self.prev_end,
            },
        }
    }

    fn unexpected_tok(expected: &str, found: Spanned<Token>) -> ParseError {
        ParseError {
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                found: found.value,
            },
            span: found.span,
        }
    }

    // ---- separators / expectations ----

    /// A statement separator: newline, end of input, or a block terminator.
    /// A terminator is left unconsumed for the enclosing construct.
    fn expect_sep(&mut self) -> Result<(), ParseError> {
        match self.peek_tok() {
            Token::Newline => {
                self.bump();
                Ok(())
            }
            Token::Eof => Ok(()),
            t if is_terminator(t) => Ok(()),
            _ => Err(self.unexpected("end of statement (newline)")),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), ParseError> {
        match self.peek_ident_value() {
            Some(s) if s == kw => {
                self.bump();
                Ok(())
            }
            _ => Err(self.unexpected(format!("`{kw}`").as_str())),
        }
    }

    fn expect_tok(&mut self, want: &Token) -> Result<(), ParseError> {
        if std::mem::discriminant(self.peek_tok()) == std::mem::discriminant(want) {
            self.bump();
            Ok(())
        } else {
            Err(self.unexpected(format!("`{want}`").as_str()))
        }
    }

    fn expect_eof(&mut self) -> Result<(), ParseError> {
        match self.peek_tok() {
            Token::Eof => Ok(()),
            _ => Err(self.unexpected("end of workflow")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.peek().cloned() {
            Some(Spanned {
                value: Token::Ident(s),
                ..
            }) => {
                self.bump();
                Ok(s)
            }
            other => Err(Self::unexpected_tok(
                "identifier",
                other.unwrap_or(Spanned {
                    value: Token::Eof,
                    span: self.prev_end..self.prev_end,
                }),
            )),
        }
    }

    // ---- block & statement parsing ----

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if self.is_eof() || self.is_block_terminator() {
                break;
            }
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_stmt(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        if matches!(self.peek_tok(), Token::At) {
            return self.parse_meta();
        }
        if let Some(kw) = self.peek_ident_value() {
            match kw {
                "if" => return self.parse_if(),
                "loop" => return self.parse_loop(),
                "for" => return self.parse_for(),
                "func" => return self.parse_func(),
                "return" => return self.parse_return(),
                "end" | "else" | "elseif" | "until" => return Err(self.unexpected("a statement")),
                _ => {}
            }
        }
        if self.is_assign_start() {
            self.parse_assign()
        } else {
            // A standalone statement must be a call; bare non-call expressions
            // (e.g. `1 + 2`, `"x"`) have no effect and are rejected.
            let e = self.parse_expr()?;
            match e.value {
                Expr::Call(call) => {
                    self.expect_sep()?;
                    Ok(self.spanned_from(start, Stmt::Call(call)))
                }
                _ => Err(self.unexpected("a call statement, e.g. `task(...)`")),
            }
        }
    }

    fn parse_meta(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        self.expect_tok(&Token::At)?;
        let mut key = self.expect_ident()?;
        while matches!(self.peek_tok(), Token::Dot) {
            self.bump();
            key.push('.');
            key.push_str(&self.expect_ident()?);
        }
        self.expect_tok(&Token::Assign)?;
        let value = self.parse_expr()?;
        self.expect_sep()?;
        Ok(self.spanned_from(
            start,
            Stmt::Meta {
                key,
                value: Box::new(value),
            },
        ))
    }

    /// `Ident =` (a single equals, not `==`) starts an assignment. Word operators
    /// (`not`/`and`/`or`/`in`) are not assignment targets.
    fn is_assign_start(&self) -> bool {
        !matches!(
            self.peek_ident_value(),
            Some(k) if is_operator_keyword(k)
        ) && matches!(self.peek_tok(), Token::Ident(_))
            && matches!(
                self.toks.get(self.i + 1).map(|s| &s.value),
                Some(Token::Assign)
            )
    }

    fn parse_assign(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        let target = self.expect_ident()?;
        self.expect_tok(&Token::Assign)?;
        let value = self.parse_expr()?;
        self.expect_sep()?;
        Ok(self.spanned_from(
            start,
            Stmt::Assign {
                target,
                value: Box::new(value),
            },
        ))
    }

    fn parse_if(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        // First branch (`if cond <block>`).
        self.expect_keyword("if")?;
        let mut branches = vec![self.parse_if_branch()?];
        // `elseif cond <block>` chain.
        while matches!(self.peek_ident_value(), Some("elseif")) {
            self.bump();
            branches.push(self.parse_if_branch()?);
        }
        // Optional trailing `else <block>`.
        let else_block = if matches!(self.peek_ident_value(), Some("else")) {
            self.bump();
            self.expect_sep()?;
            Some(self.parse_block()?)
        } else {
            None
        };
        self.expect_keyword("end")?;
        self.expect_sep()?;
        Ok(self.spanned_from(
            start,
            Stmt::If {
                branches,
                else_block,
            },
        ))
    }

    /// Parse one `cond <sep> <block>` pair. The leading `if`/`elseif` keyword
    /// is consumed by the caller.
    fn parse_if_branch(&mut self) -> Result<IfBranch, ParseError> {
        let cond = self.parse_expr()?;
        self.expect_sep()?;
        let body = self.parse_block()?;
        Ok(IfBranch {
            cond: Box::new(cond),
            body,
        })
    }

    fn parse_loop(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        self.expect_keyword("loop")?;
        // `loop` is post-test only: a body terminated by `until <cond>`.
        // A counted loop is expressed with a counter variable and `until i >= N`.
        self.expect_sep()?;
        let body = self.parse_block()?;
        self.expect_keyword("until")?;
        let cond = self.parse_expr()?;
        self.expect_sep()?;
        Ok(self.spanned_from(
            start,
            Stmt::LoopUntil {
                body,
                cond: Box::new(cond),
            },
        ))
    }

    fn parse_for(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        self.expect_keyword("for")?;
        let var = self.expect_ident()?;
        self.expect_keyword("in")?;
        let iterable = self.parse_expr()?;
        self.expect_sep()?;
        let body = self.parse_block()?;
        self.expect_keyword("end")?;
        self.expect_sep()?;
        Ok(self.spanned_from(
            start,
            Stmt::For {
                var,
                iterable: Box::new(iterable),
                body,
            },
        ))
    }

    fn parse_func(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        self.expect_keyword("func")?;
        let name = self.expect_ident()?;
        self.expect_tok(&Token::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.peek_tok(), Token::RParen) {
            loop {
                params.push(self.expect_ident()?);
                match self.peek_tok() {
                    Token::Comma => {
                        self.bump();
                    }
                    Token::RParen => break,
                    _ => return Err(self.unexpected("`,` or `)`")),
                }
            }
        }
        self.expect_tok(&Token::RParen)?;
        self.expect_sep()?;
        let body = self.parse_block()?;
        self.expect_keyword("end")?;
        self.expect_sep()?;
        Ok(self.spanned_from(start, Stmt::Function { name, params, body }))
    }

    fn parse_return(&mut self) -> Result<Spanned<Stmt>, ParseError> {
        let start = self.cur_start();
        self.expect_keyword("return")?;
        let value = match self.peek_tok() {
            Token::Newline | Token::Eof => None,
            t if is_terminator(t) => None,
            _ => Some(Box::new(self.parse_expr()?)),
        };
        self.expect_sep()?;
        Ok(self.spanned_from(start, Stmt::Return { value }))
    }

    // ---- expression parsing (precedence climbing) ----

    fn parse_expr(&mut self) -> Result<Spanned<Expr>, ParseError> {
        self.parse_or()
    }

    /// `or` (lowest precedence): `and ("or" and)*`.
    fn parse_or(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let mut lhs = self.parse_and()?;
        while matches!(self.peek_ident_value(), Some("or")) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = self.spanned_from(
                start,
                Expr::Binary {
                    op: BinaryOp::Or,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            );
        }
        Ok(lhs)
    }

    /// `and`: `not ("and" not)*`.
    fn parse_and(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let mut lhs = self.parse_not()?;
        while matches!(self.peek_ident_value(), Some("and")) {
            self.bump();
            let rhs = self.parse_not()?;
            lhs = self.spanned_from(
                start,
                Expr::Binary {
                    op: BinaryOp::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            );
        }
        Ok(lhs)
    }

    /// `not`: `"not" not | comparison` (`not` binds looser than comparisons).
    fn parse_not(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        if matches!(self.peek_ident_value(), Some("not")) {
            self.bump();
            let expr = self.parse_not()?;
            return Ok(self.spanned_from(
                start,
                Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
            ));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let lhs = self.parse_additive()?;
        if let Some(op) = cmp_op(self.peek_tok()) {
            self.bump();
            let rhs = self.parse_additive()?;
            Ok(self.spanned_from(
                start,
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            ))
        } else {
            Ok(lhs)
        }
    }

    fn parse_additive(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let mut lhs = self.parse_mul()?;
        while let Some(op) = add_op(self.peek_tok()) {
            self.bump();
            let rhs = self.parse_mul()?;
            lhs = self.spanned_from(
                start,
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            );
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let mut lhs = self.parse_unary()?;
        while let Some(op) = mul_op(self.peek_tok()) {
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = self.spanned_from(
                start,
                Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            );
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        if matches!(self.peek_tok(), Token::Minus) {
            self.bump();
            let expr = self.parse_unary()?;
            return Ok(self.spanned_from(
                start,
                Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
            ));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.cur_start();
        let tok = self.peek().cloned();
        match tok {
            Some(Spanned {
                value: Token::Int(n),
                ..
            }) => {
                self.bump();
                Ok(self.spanned_from(start, Expr::Int(n)))
            }
            Some(Spanned {
                value: Token::Float(f),
                ..
            }) => {
                self.bump();
                Ok(self.spanned_from(start, Expr::Float(f)))
            }
            Some(Spanned {
                value: Token::String(segs),
                span,
            }) => {
                self.bump();
                let parts = convert_str_segments(segs, span)?;
                Ok(self.spanned_from(start, Expr::String(parts)))
            }
            Some(Spanned {
                value: Token::LBracket,
                ..
            }) => {
                self.bump();
                let mut elems = Vec::new();
                if !matches!(self.peek_tok(), Token::RBracket) {
                    loop {
                        elems.push(self.parse_expr()?);
                        match self.peek_tok() {
                            Token::Comma => {
                                self.bump();
                            }
                            Token::RBracket => break,
                            _ => return Err(self.unexpected("`,` or `]`")),
                        }
                    }
                }
                self.expect_tok(&Token::RBracket)?;
                Ok(self.spanned_from(start, Expr::List(elems)))
            }
            Some(Spanned {
                value: Token::LParen,
                ..
            }) => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect_tok(&Token::RParen)?;
                Ok(e)
            }
            Some(Spanned {
                value: Token::Ident(name),
                ..
            }) => {
                if is_operator_keyword(&name) {
                    return Err(self.unexpected("an expression"));
                }
                self.bump();
                if matches!(self.peek_tok(), Token::LParen) {
                    self.bump();
                    let args = self.parse_args()?;
                    Ok(self.spanned_from(start, Expr::Call(Call { callee: name, args })))
                } else {
                    Ok(self.spanned_from(start, Expr::Var(name)))
                }
            }
            other => Err(Self::unexpected_tok(
                "an expression",
                other.unwrap_or(Spanned {
                    value: Token::Eof,
                    span: self.prev_end..self.prev_end,
                }),
            )),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Spanned<CallArg>>, ParseError> {
        let mut args = Vec::new();
        let mut seen_named = false;
        if matches!(self.peek_tok(), Token::RParen) {
            self.bump();
            return Ok(args);
        }
        loop {
            let arg = self.parse_arg(&mut seen_named)?;
            args.push(arg);
            match self.peek_tok() {
                Token::RParen => {
                    self.bump();
                    break;
                }
                Token::Comma => {
                    self.bump();
                }
                _ => return Err(self.unexpected("`,` or `)`")),
            }
        }
        Ok(args)
    }

    fn parse_arg(&mut self, seen_named: &mut bool) -> Result<Spanned<CallArg>, ParseError> {
        let start = self.cur_start();
        // Named argument: `Ident = expr` (single equals; `==` is a comparison).
        let is_named = matches!(self.peek_tok(), Token::Ident(_))
            && matches!(
                self.toks.get(self.i + 1).map(|s| &s.value),
                Some(Token::Assign)
            );
        if is_named {
            let name = self.expect_ident()?;
            if is_operator_keyword(&name) {
                return Err(self.unexpected("a named-argument name"));
            }
            self.expect_tok(&Token::Assign)?;
            let value = self.parse_expr()?;
            *seen_named = true;
            Ok(self.spanned_from(
                start,
                CallArg::Named {
                    name,
                    value: Box::new(value),
                },
            ))
        } else {
            if *seen_named {
                return Err(ParseError {
                    kind: ParseErrorKind::PositionalAfterNamed,
                    span: start..self.cur_start().max(start + 1),
                });
            }
            let e = self.parse_expr()?;
            Ok(self.spanned_from(start, CallArg::Positional(Box::new(e))))
        }
    }
}

// ---- free functions ----

fn is_terminator(t: &Token) -> bool {
    matches!(
        t,
        Token::Ident(s) if s == "end" || s == "else" || s == "elseif" || s == "until"
    )
}

/// Word operators that are not identifiers: they may not be used as a variable,
/// an assignment target, or a named-argument name. (`in` belongs to `for`.)
fn is_operator_keyword(s: &str) -> bool {
    matches!(s, "not" | "and" | "or" | "in")
}

fn cmp_op(t: &Token) -> Option<BinaryOp> {
    Some(match t {
        Token::Equal => BinaryOp::Eq,
        Token::NotEq => BinaryOp::Ne,
        Token::Lt => BinaryOp::Lt,
        Token::Gt => BinaryOp::Gt,
        Token::LtEq => BinaryOp::Le,
        Token::GtEq => BinaryOp::Ge,
        _ => return None,
    })
}

fn add_op(t: &Token) -> Option<BinaryOp> {
    Some(match t {
        Token::Plus => BinaryOp::Add,
        Token::Minus => BinaryOp::Sub,
        _ => return None,
    })
}

fn mul_op(t: &Token) -> Option<BinaryOp> {
    Some(match t {
        Token::Star => BinaryOp::Mul,
        Token::Slash => BinaryOp::Div,
        _ => return None,
    })
}

/// Convert lexer string segments into AST parts, parsing each interpolation's
/// inner text as an expression. Each segment carries a source span; an
/// interpolation's inner expression spans are offset back to global coordinates.
fn convert_str_segments(
    segs: Vec<Spanned<crate::lexer::StringSegment>>,
    _span: Span,
) -> Result<Vec<StringPart>, ParseError> {
    let mut out = Vec::with_capacity(segs.len());
    for seg in segs {
        match seg.value {
            crate::lexer::StringSegment::Literal(s) => out.push(StringPart::Literal(s)),
            crate::lexer::StringSegment::Interpolation(src) => {
                // The segment span covers `{...}`; the inner source range is
                // `start+1..end-1`, so the parsed inner spans shift by +1 past
                // the opening brace.
                let offset = seg.span.start + 1;
                let expr = parse_interp_expr(&src, offset)?;
                out.push(StringPart::Interpolation(Box::new(expr)));
            }
        }
    }
    Ok(out)
}

/// Parse the contents of a `{ ... }` interpolation as a single expression,
/// shifting its spans by `offset` to land on global source coordinates.
fn parse_interp_expr(src: &str, offset: usize) -> Result<Spanned<Expr>, ParseError> {
    let toks = tokenize(src).map_err(|e| ParseError {
        kind: ParseErrorKind::Lex(e.clone()),
        span: offset_span(e.span, offset),
    })?;
    let mut p = Parser::new(toks);
    let e = p.parse_expr().map_err(|e| offset_parse_error(e, offset))?;
    // The interpolation must contain exactly one expression.
    match p.peek_tok() {
        Token::Eof | Token::Newline => Ok(offset_spans(e, offset)),
        _ => Err(offset_parse_error(
            p.unexpected("end of interpolation"),
            offset,
        )),
    }
}

fn offset_span(span: Span, offset: usize) -> Span {
    (span.start + offset)..(span.end + offset)
}

fn offset_parse_error(e: ParseError, offset: usize) -> ParseError {
    ParseError {
        kind: e.kind,
        span: offset_span(e.span, offset),
    }
}

/// Recursively shift every span inside `e` by `offset`.
fn offset_spans(e: Spanned<Expr>, offset: usize) -> Spanned<Expr> {
    Spanned {
        span: offset_span(e.span, offset),
        value: offset_expr(e.value, offset),
    }
}

fn offset_expr(e: Expr, offset: usize) -> Expr {
    match e {
        Expr::List(es) => Expr::List(es.into_iter().map(|e| offset_spans(e, offset)).collect()),
        Expr::Call(c) => Expr::Call(offset_call(c, offset)),
        Expr::Binary { op, lhs, rhs } => Expr::Binary {
            op,
            lhs: Box::new(offset_spans(*lhs, offset)),
            rhs: Box::new(offset_spans(*rhs, offset)),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op,
            expr: Box::new(offset_spans(*expr, offset)),
        },
        // String literals cannot appear inside an interpolation (the lexer
        // rejects `"` there); Int/Float/Var carry no nested spans.
        Expr::Int(_) | Expr::Float(_) | Expr::Var(_) | Expr::String(_) => e,
    }
}

fn offset_call(c: Call, offset: usize) -> Call {
    Call {
        callee: c.callee,
        args: c.args.into_iter().map(|a| offset_arg(a, offset)).collect(),
    }
}

fn offset_arg(a: Spanned<CallArg>, offset: usize) -> Spanned<CallArg> {
    let value = match a.value {
        CallArg::Positional(e) => CallArg::Positional(Box::new(offset_spans(*e, offset))),
        CallArg::Named { name, value } => CallArg::Named {
            name,
            value: Box::new(offset_spans(*value, offset)),
        },
    };
    Spanned {
        span: offset_span(a.span, offset),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Module {
        parse(src).unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"))
    }

    fn first_stmt(m: &Module) -> &Spanned<Stmt> {
        m.body.first().expect("at least one statement")
    }

    #[test]
    fn assignment_int() {
        let m = parse_ok("x = 5\n");
        match &first_stmt(&m).value {
            Stmt::Assign { target, value } => {
                assert_eq!(target, "x");
                assert_eq!(value.value, Expr::Int(5));
            }
            other => panic!("unexpected stmt {other:?}"),
        }
    }

    #[test]
    fn assignment_string_interpolation() {
        let m = parse_ok("s = \"a {y} b\"\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!();
        };
        let Expr::String(parts) = &value.value else {
            panic!();
        };
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], StringPart::Literal(s) if s == "a "));
        match &parts[1] {
            StringPart::Interpolation(e) => assert_eq!(e.value, Expr::Var("y".into())),
            StringPart::Literal(_) => panic!("expected an interpolation segment"),
        }
        assert!(matches!(&parts[2], StringPart::Literal(s) if s == " b"));
    }

    #[test]
    fn call_with_named_args() {
        let m = parse_ok("task(\"do it\", return=\"path\", check=\"ok\", produce=[\"a\"])\n");
        let Stmt::Call(call) = &first_stmt(&m).value else {
            panic!()
        };
        assert_eq!(call.callee, "task");
        assert_eq!(call.args.len(), 4);
        // positional first
        assert!(matches!(call.args[0].value, CallArg::Positional(_)));
        // then named
        let names: Vec<Option<&str>> = call.args.iter().map(|a| a.value.name()).collect();
        assert_eq!(
            names,
            vec![None, Some("return"), Some("check"), Some("produce")]
        );
    }

    #[test]
    fn multiline_call() {
        let m = parse_ok("task(\"a\",\n     return=\"b\")\n");
        let Stmt::Call(call) = &first_stmt(&m).value else {
            panic!()
        };
        assert_eq!(call.args.len(), 2);
    }

    #[test]
    fn meta_directive_parses_key() {
        let m = parse_ok("@template = \"bugfix\"\n");
        let Stmt::Meta { key, value } = &first_stmt(&m).value else {
            panic!("expected meta directive")
        };
        assert_eq!(key, "template");
        assert!(matches!(value.value, Expr::String(_)));
    }

    #[test]
    fn bare_non_call_statement_is_rejected() {
        // A standalone statement must be a call; pure expressions are no-ops
        // and are rejected.
        assert!(parse("1 + 2\n").is_err());
        assert!(parse("\"hello\"\n").is_err());
        assert!(parse("x == y\n").is_err());
        assert!(parse("[1, 2, 3]\n").is_err());
    }

    #[test]
    fn operator_keywords_are_not_identifiers() {
        // `not`/`and`/`or`/`in` are word operators, not usable as variables,
        // assignment targets, or named-argument names.
        assert!(parse("x = not\n").is_err());
        assert!(parse("not = 5\n").is_err());
        assert!(parse("task(not = 1)\n").is_err());
        assert!(parse("x = a or\n").is_err());
        // ... but they still work as operators.
        parse_ok("x = a or b\n");
        parse_ok("x = not a\n");
    }

    #[test]
    fn if_else_end() {
        let m = parse_ok("if x > 3\n  print(\"big\")\nelse\n  print(\"small\")\nend\n");
        let Stmt::If {
            branches,
            else_block,
        } = &first_stmt(&m).value
        else {
            panic!()
        };
        assert_eq!(branches.len(), 1);
        assert!(matches!(
            branches[0].cond.value,
            Expr::Binary {
                op: BinaryOp::Gt,
                ..
            }
        ));
        assert_eq!(branches[0].body.len(), 1);
        assert!(else_block.is_some());
    }

    #[test]
    fn if_without_else() {
        let m = parse_ok("if x > 3\n  print(\"big\")\nend\n");
        let Stmt::If {
            branches,
            else_block,
        } = &first_stmt(&m).value
        else {
            panic!()
        };
        assert_eq!(branches.len(), 1);
        assert!(else_block.is_none());
    }

    #[test]
    fn if_elseif_else_parses_to_branches() {
        let m =
            parse_ok("if a\n  print(\"1\")\nelseif b\n  print(\"2\")\nelse\n  print(\"3\")\nend\n");
        let Stmt::If {
            branches,
            else_block,
        } = &first_stmt(&m).value
        else {
            panic!()
        };
        assert_eq!(branches.len(), 2); // the `if` plus one `elseif`
        assert_eq!(branches[0].cond.value, Expr::Var("a".into()));
        assert_eq!(branches[1].cond.value, Expr::Var("b".into()));
        assert!(else_block.is_some());
    }

    #[test]
    fn loop_until() {
        let m = parse_ok("loop\n  x = x + 1\nuntil x > 5\n");
        let Stmt::LoopUntil { body, cond } = &first_stmt(&m).value else {
            panic!()
        };
        assert_eq!(body.len(), 1);
        assert!(matches!(
            cond.value,
            Expr::Binary {
                op: BinaryOp::Gt,
                ..
            }
        ));
    }

    #[test]
    fn loop_is_post_test_only_no_counted_form() {
        // `loop` is post-test only; a counted `loop N ... end` is rejected.
        assert!(parse("loop 3\n  print(\"hi\")\nend\n").is_err());
        // the old `repeat` keyword is gone
        assert!(parse("repeat\n  print(\"hi\")\nuntil x > 5\n").is_err());
    }

    #[test]
    fn logical_precedence_or_binds_looser_than_and() {
        // `a or b and c` groups as `a or (b and c)`.
        let m = parse_ok("x = a or b and c\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!()
        };
        let Expr::Binary {
            op: BinaryOp::Or,
            rhs,
            ..
        } = &value.value
        else {
            panic!("top-level operator must be Or");
        };
        assert!(matches!(
            &rhs.value,
            Expr::Binary {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn not_binds_looser_than_comparison() {
        // `not a == b` groups as `not (a == b)`.
        let m = parse_ok("x = not a == b\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!()
        };
        let Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } = &value.value
        else {
            panic!("top-level must be Not");
        };
        assert!(matches!(
            &expr.value,
            Expr::Binary {
                op: BinaryOp::Eq,
                ..
            }
        ));
    }

    #[test]
    fn for_in() {
        let m = parse_ok("for item in list\n  print(item)\nend\n");
        let Stmt::For {
            var,
            iterable,
            body,
        } = &first_stmt(&m).value
        else {
            panic!()
        };
        assert_eq!(var, "item");
        assert_eq!(iterable.value, Expr::Var("list".into()));
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn func_def() {
        let m = parse_ok("func add(a, b)\n  return a + b\nend\n");
        let Stmt::Function { name, params, body } = &first_stmt(&m).value else {
            panic!()
        };
        assert_eq!(name, "add");
        assert_eq!(params.clone(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].value, Stmt::Return { .. }));
    }

    #[test]
    fn return_bare_and_value() {
        let m = parse_ok("return\n");
        let Stmt::Return { value } = &first_stmt(&m).value else {
            panic!()
        };
        assert!(value.is_none());

        let m = parse_ok("return a + b\n");
        let Stmt::Return { value } = &first_stmt(&m).value else {
            panic!()
        };
        assert!(value.is_some());
    }

    #[test]
    fn precedence_add_mul() {
        let m = parse_ok("x = 1 + 2 * 3\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!()
        };
        // expected: 1 + (2 * 3)
        let Expr::Binary { op, lhs, rhs } = &value.value else {
            panic!()
        };
        assert_eq!(*op, BinaryOp::Add);
        assert_eq!(lhs.value, Expr::Int(1));
        assert!(matches!(
            rhs.value,
            Expr::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn list_literal() {
        let m = parse_ok("x = [\"a\", \"b\", \"c\"]\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!()
        };
        let Expr::List(elems) = &value.value else {
            panic!()
        };
        assert_eq!(elems.len(), 3);
    }

    #[test]
    fn parens_grouping() {
        let m = parse_ok("x = (1 + 2) * 3\n");
        let Stmt::Assign { value, .. } = &first_stmt(&m).value else {
            panic!()
        };
        let Expr::Binary { op, .. } = &value.value else {
            panic!()
        };
        assert_eq!(*op, BinaryOp::Mul);
    }

    #[test]
    fn nested_blocks() {
        let src = "for f in files\n  if f == \"a\"\n    print(f)\n  end\nend\n";
        let m = parse_ok(src);
        let Stmt::For { body, .. } = &first_stmt(&m).value else {
            panic!()
        };
        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].value, Stmt::If { .. }));
    }

    #[test]
    fn err_missing_end() {
        let err = parse("if x > 3\n  print(\"a\")\n").unwrap_err();
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedToken { ref expected, .. } if expected.contains("end")
        ));
    }

    #[test]
    fn err_unexpected_token() {
        let err = parse("x =\n").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::UnexpectedToken { .. }));
    }

    #[test]
    fn err_positional_after_named() {
        let err = parse("task(return=\"x\", \"pos\")\n").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::PositionalAfterNamed);
    }

    #[test]
    fn err_stray_end_at_top_level() {
        let err = parse("x = 1\nend\n").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::UnexpectedToken { .. }));
    }

    #[test]
    fn err_lex_error_propagates() {
        let err = parse("x = \"unterminated").unwrap_err();
        assert!(matches!(err.kind, ParseErrorKind::Lex(_)));
    }

    #[test]
    fn root_cause_bugfix_workflow_parses() {
        // The canonical example from IDEA.md (lightly trimmed) should parse.
        let src = r#"
func analyze(bug)
    existing = command("test -f root-{bug}.md", return="yes or no")
    if existing == "yes"
        return "root-{bug}.md"
    end
    task("analyze root cause", return="path", produce=["root-{bug}.md"],
         check="contains root")
    return "root-{bug}.md"
end

rc = analyze("login-500")
print("root at {rc}")

gate = ask("continue?", return="yes or no")
if gate != "yes"
    return
end

files = command("git diff --name-only", return="list of files")
for f in files
    task("fix {f}", return="bool", check="ok")
end

print("done")
"#;
        let m = parse_ok(src);
        assert!(m.body.len() >= 5);
    }

    #[test]
    fn spans_are_populated() {
        let m = parse_ok("x = 5\n");
        let s = &m.body[0];
        assert!(s.span.start <= s.span.end);
    }
}
