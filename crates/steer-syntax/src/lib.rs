//! Lexical and syntactic front-end for the steer workflow DSL.
//!
//! This crate contains the lexer, parser and AST. It is deliberately free of
//! I/O and of the steer runtime, so it can be reused by tooling such as a
//! future language server.

#![forbid(unsafe_code)]
// Lint policy: the pedantic lints allowed below are deliberate
// project choices, documented here so they are not mistaken for oversights.
#![allow(
    clippy::cast_possible_truncation, // token spans and lexer indices are bounded
    clippy::cast_precision_loss,      // i64 -> f64 in numeric evaluation is intended
    clippy::module_name_repetitions,  // re-exports keep call-site names explicit
    clippy::wildcard_imports,         // `use crate::ast::*` is intentional AST ergonomics
    clippy::many_single_char_names,   // short loop counters and match binders (n, s, t)
    clippy::must_use_candidate,       // `#[must_use]` is added only where dropping is a bug
    clippy::single_char_pattern,      // single-char string patterns read fine in the lexer
    clippy::single_match_else,        // a two-arm `match` is often clearer than `if/else`
    clippy::if_not_else,              // condition polarity follows the surrounding logic
    clippy::map_unwrap_or             // `.map(f).unwrap_or(a)` is preferred here for order
)]

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod source;

pub use ast::*;
pub use lexer::{line_col, tokenize, LexError, LexErrorKind, StringSegment, Token};
pub use parser::{parse, ParseError, ParseErrorKind, Parser};
pub use source::{Span, Spanned};
