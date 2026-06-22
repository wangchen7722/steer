//! Source-location infrastructure shared by the lexer, parser, and AST.
//!
//! [`Span`] is a byte range into the source text; [`Spanned`] pairs any value
//! with the range it occupies, so diagnostics — and a future language server —
//! can point at the originating text. Defining these here (rather than in the
//! lexer) keeps the dependency direction clean: the AST depends on source
//! infrastructure, not on the lexer.

/// A half-open byte span `[start, end)` into the source text.
pub type Span = std::ops::Range<usize>;

/// A value paired with the source span it occupies.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    /// The wrapped value.
    pub value: T,
    /// The source range the value was parsed from.
    pub span: Span,
}
