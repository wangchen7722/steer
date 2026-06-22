# 2. File and Module Organization

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

A source file SHOULD follow this order when the sections are present:

1. module-level documentation (`//!`);
2. imports;
3. constants and type aliases;
4. public types and traits;
5. private helper types;
6. trait implementations and inherent implementations;
7. free functions;
8. tests.

For example, a `value.rs` module would read: module doc, imports, the `Value` enum and `EvalError`
type, `impl Value` / `impl Display` / `impl Error`, the free functions (`eval_literal`, `eval`,
helpers), and finally `#[cfg(test)] mod tests`.

```rust
//! Runtime values and expression evaluation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ast::{BinOp, Expr, StrPart, UnaryOp};
use crate::Spanned;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
}

impl Value {
    pub fn truthy(&self) -> bool {
        // ...
    }
}

/// Evaluate an expression against the current variable scope.
pub fn eval(expr: &Expr, vars: &HashMap<String, Value>) -> Result<Value, EvalError> {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    // ...
}
```

## Rules

- Imports **MUST** appear before module declarations.
- Group imports with one blank line between standard-library, external-crate, and crate-local
  imports — `std::…`, then third-party crates (e.g. `serde::…`), then local crates
  (e.g. `crate::…`).
- Prefer explicit imports over glob imports.
- `use super::*;` is allowed inside a small `#[cfg(test)]` module when it materially improves
  test readability.
- New modules SHOULD use `foo.rs` instead of `foo/mod.rs` unless a directory module is clearly
  more readable. Keep each concern in its own file.
- Keep a module focused on one domain concern. Split a module when it mixes unrelated
  responsibilities such as parsing, execution, CLI, persistence, and rendering.

```rust
// Incorrect: the glob hides which items a module actually depends on.
use crate::ast::*;

// Correct
use crate::ast::{Arg, BinOp, Expr, Stmt};
```

```rust
// Incorrect: one crate mixes every layer.
mod app {
    // domain model definitions
    // parser implementation
    // CLI argument parsing
    // runtime + interpreter
    // filesystem persistence
}

// Better: split by responsibility across crates and modules, e.g.
//   crates/myapp-syntax -> ast, lexer, parser        (parsing only, no I/O)
//   crates/myapp-core   -> context, ir, vm, value    (runtime; no I/O except storage)
//   crates/myapp-cli    -> the binary                (CLI + orchestration)
```
