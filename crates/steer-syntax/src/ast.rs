//! Abstract syntax for the steer workflow DSL.
//!
//! The AST is produced by [`crate::parser`] and consumed by validation and the
//! interpreter. Every node carries a source [`Span`](crate::Span) so diagnostics and a future
//! language server can point at the originating text.

use crate::source::Spanned;

/// A parsed workflow module: a top-level block of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub body: Block,
}

/// A sequence of statements: a block body, a function body, or the top level.
pub type Block = Vec<Spanned<Stmt>>;

/// One condition/body pair in an `if`/`elseif` chain.
#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    /// The condition of this `if` or `elseif` branch.
    pub cond: Box<Spanned<Expr>>,
    /// The block executed when `cond` is truthy.
    pub body: Block,
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `@key = value` — a workflow-level runtime directive.
    Meta {
        key: String,
        value: Box<Spanned<Expr>>,
    },
    /// `target = value`
    Assign {
        target: String,
        value: Box<Spanned<Expr>>,
    },
    /// A call used as a standalone statement for its effects, e.g. `task(...)`.
    Call(Call),
    /// `if c1 ... elseif c2 ... else ... end` — an explicit branch chain.
    /// `branches` holds the `if` branch first, then each `elseif` condition/body
    /// in source order; `else_block` is the optional trailing `else`.
    If {
        branches: Vec<IfBranch>,
        else_block: Option<Block>,
    },
    /// `loop ... until cond` — post-test, so the body always runs at least once.
    /// A fixed iteration count is expressed with a counter variable plus
    /// `until i >= N`; there is no separate counted-loop form.
    LoopUntil {
        body: Block,
        cond: Box<Spanned<Expr>>,
    },
    /// `for var in iterable ... end`
    For {
        var: String,
        iterable: Box<Spanned<Expr>>,
        body: Block,
    },
    /// `func name(params) ... end`
    Function {
        name: String,
        params: Vec<String>,
        body: Block,
    },
    /// `return [value]`
    Return { value: Option<Box<Spanned<Expr>>> },
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    /// A string literal, possibly with `{expr}` interpolations.
    String(Vec<StringPart>),
    /// A list literal `[a, b, c]`.
    List(Vec<Spanned<Expr>>),
    /// A bare identifier naming a variable.
    Var(String),
    /// A function / action-node call.
    Call(Call),
    /// A binary operation such as `+`, `==`, or `<`.
    Binary {
        op: BinaryOp,
        lhs: Box<Spanned<Expr>>,
        rhs: Box<Spanned<Expr>>,
    },
    /// A unary operation; currently only numeric negation `-expr`.
    Unary {
        op: UnaryOp,
        expr: Box<Spanned<Expr>>,
    },
}

/// A segment of a string literal.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// A literal run of characters.
    Literal(String),
    /// An interpolation `{ expr }`, parsed into an expression.
    Interpolation(Box<Spanned<Expr>>),
}

/// A call to a function or action node.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The callee name, e.g. `task`, `ask`, `command`, or a user `func`.
    pub callee: String,
    /// Positional arguments first, then named arguments.
    pub args: Vec<Spanned<CallArg>>,
}

/// A call argument: positional or named.
#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    /// A positional argument.
    Positional(Box<Spanned<Expr>>),
    /// A named argument such as `return="..."`.
    Named {
        name: String,
        value: Box<Spanned<Expr>>,
    },
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Numeric negation, written `-`.
    Neg,
    /// Logical `not`: truthiness inversion.
    Not,
}

impl CallArg {
    /// The name of a named argument, or `None` for positional ones.
    pub fn name(&self) -> Option<&str> {
        match self {
            CallArg::Named { name, .. } => Some(name),
            CallArg::Positional(_) => None,
        }
    }
}
