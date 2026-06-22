//! Runtime values and expression evaluation.
//!
//! Values feed the template engine's context and live in the execution context.
//! In a real run they come from the agent via `steer set`; for static rendering
//! (validation / simulate) they are taken directly from a call's literal
//! arguments via [`eval_literal`], where variables degrade to a `{name}`
//! placeholder. At run time, [`eval`] resolves variables and computes
//! arithmetic and comparisons against the current scope.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use steer_syntax::ast::{BinaryOp, Expr, StringPart, UnaryOp};
use steer_syntax::Spanned;

/// A runtime value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Absent / unset.
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    /// Truthiness used by `{% if %}` and `if`/`until` conditions: null/empty/
    /// false is false; every other value — including numeric zero — is true.
    /// (For "is it set / non-zero?" use an explicit comparison.)
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(_) | Value::Float(_) => true,
            Value::Str(s) => !s.is_empty(),
            Value::List(v) => !v.is_empty(),
            Value::Object(v) => !v.is_empty(),
        }
    }

    /// Render to plain text for `{{ }}` interpolation.
    pub fn render(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(x) => x.to_string(),
            Value::Str(s) => s.clone(),
            Value::List(v) => v.iter().map(Value::render).collect::<Vec<_>>().join(", "),
            Value::Object(_) => serde_json::to_string(self).unwrap_or_default(),
        }
    }
}

/// An evaluation error.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// A variable was referenced before it was set.
    UnsetVar(String),
    /// An operand had the wrong type for an operator.
    TypeError(String),
    /// Integer overflow or division by zero.
    Arithmetic(String),
    /// A call expression appeared where a value was expected. Calls are
    /// separate instructions, never sub-expressions.
    UnexpectedCall,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UnsetVar(v) => write!(f, "variable `{v}` is not set"),
            EvalError::TypeError(m) => write!(f, "type error: {m}"),
            EvalError::Arithmetic(m) => write!(f, "arithmetic error: {m}"),
            EvalError::UnexpectedCall => write!(f, "unexpected call in expression"),
        }
    }
}

impl std::error::Error for EvalError {}

// ---- static evaluation (placeholders for unknowns) ----

/// Evaluate a literal expression to a [`Value`] for static rendering.
///
/// `true` / `false` are booleans; other variables and computed expressions
/// degrade to a placeholder.
pub fn eval_literal(e: &Spanned<Expr>) -> Value {
    match &e.value {
        Expr::Int(n) => Value::Int(*n),
        Expr::Float(f) => Value::Float(*f),
        Expr::String(parts) => {
            let mut s = String::new();
            for p in parts {
                match p {
                    StringPart::Literal(t) => s.push_str(t),
                    StringPart::Interpolation(inner) => s.push_str(&interp_placeholder(inner)),
                }
            }
            Value::Str(s)
        }
        Expr::List(elems) => Value::List(elems.iter().map(eval_literal).collect()),
        Expr::Var(name) => match name.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::Str(format!("{{{name}}}")),
        },
        Expr::Binary { .. } | Expr::Unary { .. } | Expr::Call(_) => Value::Str("{...}".into()),
    }
}

fn interp_placeholder(e: &Spanned<Expr>) -> String {
    match &e.value {
        Expr::Var(name) => format!("{{{name}}}"),
        Expr::Int(n) => n.to_string(),
        Expr::Float(f) => f.to_string(),
        _ => "{...}".to_string(),
    }
}

/// Parse a `steer set` value literal.
///
/// JSON values become typed values: `[1,2,3]` -> list, `42` -> int, `true`
/// -> bool, `"..."` -> string, and objects -> maps. A bare word that is not
/// valid JSON is treated as a string.
pub fn parse_value(s: &str) -> Value {
    match serde_json::from_str::<serde_json::Value>(s) {
        Err(_) => Value::Str(s.to_string()),
        Ok(jv) => json_to_value(jv),
    }
}

fn json_to_value(jv: serde_json::Value) -> Value {
    match jv {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(a) => Value::List(a.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(key, value)| (key, json_to_value(value)))
                .collect(),
        ),
    }
}

// ---- runtime evaluation (real values) ----

/// Evaluate an expression against the current variable scope.
///
/// # Errors
/// Returns an [`EvalError`] for unset variables, type mismatches, or stray
/// call expressions.
pub fn eval(expr: &Expr, vars: &HashMap<String, Value>) -> Result<Value, EvalError> {
    match expr {
        Expr::Int(n) => Ok(Value::Int(*n)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::String(parts) => {
            let mut s = String::new();
            for p in parts {
                match p {
                    StringPart::Literal(t) => s.push_str(t),
                    StringPart::Interpolation(e) => s.push_str(&eval(&e.value, vars)?.render()),
                }
            }
            Ok(Value::Str(s))
        }
        Expr::List(es) => Ok(Value::List(
            es.iter()
                .map(|e| eval(&e.value, vars))
                .collect::<Result<_, _>>()?,
        )),
        Expr::Var(name) => match name.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => vars
                .get(name)
                .cloned()
                .ok_or_else(|| EvalError::UnsetVar(name.clone())),
        },
        Expr::Binary { op, lhs, rhs } => match op {
            // Short-circuit logical operators: the RHS is only evaluated when
            // the LHS does not already decide the result.
            BinaryOp::And => {
                if !eval(&lhs.value, vars)?.truthy() {
                    return Ok(Value::Bool(false));
                }
                Ok(Value::Bool(eval(&rhs.value, vars)?.truthy()))
            }
            BinaryOp::Or => {
                if eval(&lhs.value, vars)?.truthy() {
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(eval(&rhs.value, vars)?.truthy()))
            }
            _ => {
                let l = eval(&lhs.value, vars)?;
                let r = eval(&rhs.value, vars)?;
                apply_binop(*op, &l, &r)
            }
        },
        Expr::Unary { op, expr } => {
            let v = eval(&expr.value, vars)?;
            apply_unop(*op, &v)
        }
        Expr::Call(_) => Err(EvalError::UnexpectedCall),
    }
}

#[derive(Clone, Copy)]
enum Num {
    I(i64),
    F(f64),
}

impl Num {
    fn as_f64(self) -> f64 {
        match self {
            Num::I(n) => n as f64,
            Num::F(x) => x,
        }
    }
}

fn as_num(v: &Value) -> Result<Num, EvalError> {
    match v {
        Value::Int(n) => Ok(Num::I(*n)),
        Value::Float(f) => Ok(Num::F(*f)),
        _ => Err(EvalError::TypeError(format!(
            "expected a number, found {}",
            v.render()
        ))),
    }
}

fn apply_binop(op: BinaryOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
    use BinaryOp::*;
    match op {
        Add | Sub | Mul | Div => {
            let a = as_num(l)?;
            let b = as_num(r)?;
            match (a, b) {
                (Num::I(x), Num::I(y)) => {
                    let v = match op {
                        Add => x.checked_add(y),
                        Sub => x.checked_sub(y),
                        Mul => x.checked_mul(y),
                        Div => {
                            if y == 0 {
                                return Err(EvalError::Arithmetic("division by zero".into()));
                            }
                            x.checked_div(y)
                        }
                        _ => unreachable!("narrowed by the outer match arm"),
                    };
                    let v = v.ok_or_else(|| EvalError::Arithmetic("integer overflow".into()))?;
                    Ok(Value::Int(v))
                }
                (x, y) => {
                    let (a, b) = (x.as_f64(), y.as_f64());
                    let v = match op {
                        Add => a + b,
                        Sub => a - b,
                        Mul => a * b,
                        Div => {
                            if b == 0.0 {
                                return Err(EvalError::Arithmetic("division by zero".into()));
                            }
                            a / b
                        }
                        _ => unreachable!("narrowed by the outer match arm"),
                    };
                    Ok(Value::Float(v))
                }
            }
        }
        Eq => values_eq(l, r).map(Value::Bool),
        Ne => values_eq(l, r).map(|eq| Value::Bool(!eq)),
        Lt | Gt | Le | Ge => {
            let c = compare(l, r)?;
            Ok(Value::Bool(match op {
                Lt => c < 0,
                Gt => c > 0,
                Le => c <= 0,
                Ge => c >= 0,
                _ => unreachable!("narrowed by the outer match arm"),
            }))
        }
        // Logical operators are short-circuit evaluated in `eval` and never
        // reach `apply_binop`.
        And | Or => unreachable!("logical operators are short-circuit evaluated"),
    }
}

fn apply_unop(op: UnaryOp, v: &Value) -> Result<Value, EvalError> {
    match op {
        UnaryOp::Neg => match as_num(v)? {
            Num::I(n) => n
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| EvalError::Arithmetic("integer overflow".into())),
            Num::F(x) => Ok(Value::Float(-x)),
        },
        UnaryOp::Not => Ok(Value::Bool(!v.truthy())),
    }
}

/// Value equality: equal types compare by value; two numbers compare by
/// magnitude. Mismatched, non-numeric types are an error (`1 == "1"` is a type
/// error, not silently false), matching [`compare`].
fn values_eq(l: &Value, r: &Value) -> Result<bool, EvalError> {
    match (l, r) {
        (Value::Int(a), Value::Int(b)) => Ok(a == b),
        (Value::Float(a), Value::Float(b)) => Ok(a == b),
        (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
            Ok((*a as f64) == *b)
        }
        (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
        (Value::Str(a), Value::Str(b)) => Ok(a == b),
        (Value::Null, Value::Null) => Ok(true),
        _ => Err(EvalError::TypeError(format!(
            "cannot compare {} and {} for equality",
            l.render(),
            r.render()
        ))),
    }
}

/// Three-way comparison for `<`, `>`, etc. Numbers compare numerically; strings
/// lexicographically; mismatched types are an error.
fn compare(l: &Value, r: &Value) -> Result<i8, EvalError> {
    use std::cmp::Ordering;
    let ord: Ordering = match (l, r) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        _ => {
            return Err(EvalError::TypeError(format!(
                "cannot compare {} and {}",
                l.render(),
                r.render()
            )))
        }
    };
    Ok(match ord {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use steer_syntax::parse;

    fn expr_of(src: &str) -> Expr {
        let m = parse(&format!("__x = {src}\n")).unwrap();
        match &m.body[0].value {
            steer_syntax::ast::Stmt::Assign { value, .. } => value.value.clone(),
            _ => unreachable!(),
        }
    }

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
            .collect()
    }

    #[test]
    fn eval_int_float_string() {
        assert_eq!(eval(&expr_of("42"), &HashMap::new()), Ok(Value::Int(42)));
        assert_eq!(
            eval(&expr_of("1.5"), &HashMap::new()),
            Ok(Value::Float(1.5))
        );
    }

    #[test]
    fn eval_var_lookup() {
        let mut v = HashMap::new();
        v.insert("name".to_string(), Value::Str("bob".into()));
        assert_eq!(eval(&expr_of("name"), &v), Ok(Value::Str("bob".into())));
    }

    #[test]
    fn eval_unset_var_is_error() {
        assert_eq!(
            eval(&expr_of("missing"), &HashMap::new()),
            Err(EvalError::UnsetVar("missing".into()))
        );
    }

    #[test]
    fn eval_arithmetic() {
        assert_eq!(
            eval(&expr_of("1 + 2 * 3"), &HashMap::new()),
            Ok(Value::Int(7))
        );
        assert_eq!(eval(&expr_of("7 / 2"), &HashMap::new()), Ok(Value::Int(3)));
        assert_eq!(
            eval(&expr_of("7.0 / 2"), &HashMap::new()),
            Ok(Value::Float(3.5))
        );
    }

    #[test]
    fn eval_div_by_zero() {
        assert!(matches!(
            eval(&expr_of("1 / 0"), &HashMap::new()),
            Err(EvalError::Arithmetic(_))
        ));
    }

    #[test]
    fn eval_string_equality() {
        let v = vars(&[("gate", "yes")]);
        assert_eq!(eval(&expr_of("gate == \"yes\""), &v), Ok(Value::Bool(true)));
        assert_eq!(
            eval(&expr_of("gate != \"yes\""), &v),
            Ok(Value::Bool(false))
        );
    }

    #[test]
    fn eval_numeric_comparison() {
        assert_eq!(
            eval(&expr_of("3 > 2"), &HashMap::new()),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            eval(&expr_of("3 < 2"), &HashMap::new()),
            Ok(Value::Bool(false))
        );
    }

    #[test]
    fn eval_string_interpolation_resolves() {
        let mut v = HashMap::new();
        v.insert("who".to_string(), Value::Str("world".into()));
        assert_eq!(
            eval(&expr_of("\"hi {who}\""), &v),
            Ok(Value::Str("hi world".into()))
        );
    }

    #[test]
    fn eval_literal_keeps_placeholder() {
        // static path keeps {x} since there is no scope
        let m = parse("__x = \"a {x}\"\n").unwrap();
        let e = match &m.body[0].value {
            steer_syntax::ast::Stmt::Assign { value, .. } => value.clone(),
            _ => unreachable!(),
        };
        assert_eq!(eval_literal(&e), Value::Str("a {x}".into()));
    }

    #[test]
    fn truthiness() {
        assert!(Value::Str("x".into()).truthy());
        assert!(!Value::Null.truthy());
        assert!(!Value::Bool(false).truthy());
    }

    #[test]
    fn eval_and_or_not() {
        assert_eq!(
            eval(&expr_of("true and false"), &HashMap::new()),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            eval(&expr_of("true and true"), &HashMap::new()),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            eval(&expr_of("true or false"), &HashMap::new()),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            eval(&expr_of("false or false"), &HashMap::new()),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            eval(&expr_of("not true"), &HashMap::new()),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            eval(&expr_of("not false"), &HashMap::new()),
            Ok(Value::Bool(true))
        );
    }

    #[test]
    fn eval_logical_short_circuits() {
        // `missing` is unset; it would error if evaluated. `and` short-circuits
        // on a falsy LHS, `or` on a truthy LHS, so the RHS is never evaluated.
        assert_eq!(
            eval(&expr_of("false and missing"), &HashMap::new()),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            eval(&expr_of("true or missing"), &HashMap::new()),
            Ok(Value::Bool(true))
        );
    }

    #[test]
    fn parse_value_typed_literals() {
        assert_eq!(
            parse_value("[1, 2, 3]"),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(parse_value("42"), Value::Int(42));
        assert_eq!(parse_value("true"), Value::Bool(true));
        assert_eq!(parse_value("\"hi\""), Value::Str("hi".into()));
        // a quoted string that looks like an array stays a string
        assert_eq!(parse_value("\"[1, 2, 3]\""), Value::Str("[1, 2, 3]".into()));
        // a bare word is not JSON -> treated as a string
        assert_eq!(parse_value("hello"), Value::Str("hello".into()));
    }
}
