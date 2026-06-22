//! Semantic validation of a parsed steer workflow.
//!
//! [`validate`] takes a parsed [`Module`] and returns every diagnostic found,
//! so authors see all problems at once rather than fixing them one at a time.
//! Parsing itself is handled by [`steer_syntax::parse`]; this module concerns
//! itself with rules the parser cannot express (types of named arguments,
//! value-task requirements, duplicate names, ...).

use std::collections::{HashMap, HashSet};

use steer_syntax::ast::{Call, CallArg, Expr, Stmt, StringPart};
use steer_syntax::{Module, Span, Spanned};

use crate::template::{resolve_template, ParamKind};

/// Diagnostic severity. Only errors are produced in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
}

/// A validation diagnostic with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            span,
            message: message.into(),
        }
    }
}

/// `checked` is the per-op verification flag (set via `steer set checked`), and
/// `__`-prefixed names back the lowering's hidden loop/return slots; neither may
/// be bound by workflow code.
fn is_reserved_binding(name: &str) -> bool {
    name == "checked" || name.starts_with("__")
}

/// Run all semantic checks on `module`, returning every diagnostic found.
pub fn validate(module: &Module) -> Vec<Diagnostic> {
    let mut v = Validator::default();
    v.collect_funcs(&module.body, true);
    v.visit_block(&module.body);
    v.diags
}

#[derive(Default)]
struct Validator {
    diags: Vec<Diagnostic>,
    funcs: HashMap<String, Span>,
}

impl Validator {
    /// Collect function definitions for call resolution. `top_level` is true
    /// only for the module body; functions nested inside `if`/`loop`/`for`/
    /// `func` are rejected because the IR only emits entry points for
    /// top-level definitions (a nested call would otherwise render as an
    /// agent op instead of executing).
    fn collect_funcs(&mut self, block: &[Spanned<Stmt>], top_level: bool) {
        for s in block {
            match &s.value {
                Stmt::Function {
                    name, params, body, ..
                } => {
                    if !top_level {
                        self.diags.push(Diagnostic::error(
                            s.span.clone(),
                            "functions must be defined at the top level",
                        ));
                    }
                    let mut seen = HashSet::new();
                    for p in params {
                        if !seen.insert(p.clone()) {
                            self.diags.push(Diagnostic::error(
                                s.span.clone(),
                                format!("duplicate parameter `{p}` in function `{name}`"),
                            ));
                        }
                    }
                    if self.funcs.insert(name.clone(), s.span.clone()).is_some() {
                        self.diags.push(Diagnostic::error(
                            s.span.clone(),
                            format!("duplicate function `{name}`"),
                        ));
                    }
                    self.collect_funcs(body, false);
                }
                Stmt::If {
                    branches,
                    else_block,
                } => {
                    for b in branches {
                        self.collect_funcs(&b.body, false);
                    }
                    if let Some(eb) = else_block {
                        self.collect_funcs(eb, false);
                    }
                }
                Stmt::LoopUntil { body, .. } | Stmt::For { body, .. } => {
                    self.collect_funcs(body, false);
                }
                _ => {}
            }
        }
    }

    fn visit_block(&mut self, block: &[Spanned<Stmt>]) {
        for s in block {
            self.visit_stmt(s);
        }
    }

    fn visit_stmt(&mut self, s: &Spanned<Stmt>) {
        match &s.value {
            Stmt::Meta { value, .. } => self.visit_expr(value),
            Stmt::Assign { target, value } => {
                self.check_binding(s.span.clone(), target);
                self.check_value_expr(value);
                self.visit_expr(value);
            }
            Stmt::Call(call) => {
                self.check_call(s.span.clone(), call);
                for a in &call.args {
                    let inner = match &a.value {
                        CallArg::Positional(x) | CallArg::Named { value: x, .. } => x,
                    };
                    self.visit_expr(inner);
                }
            }
            Stmt::Return { value } => {
                if let Some(v) = value {
                    self.check_value_expr(v);
                    self.visit_expr(v);
                }
            }
            Stmt::If {
                branches,
                else_block,
            } => {
                for b in branches {
                    self.visit_expr(&b.cond);
                    self.visit_block(&b.body);
                }
                if let Some(eb) = else_block {
                    self.visit_block(eb);
                }
            }
            Stmt::LoopUntil { body, cond } => {
                self.visit_block(body);
                self.visit_expr(cond);
            }
            Stmt::For {
                var,
                iterable,
                body,
            } => {
                self.check_binding(s.span.clone(), var);
                self.visit_expr(iterable);
                self.visit_block(body);
            }
            Stmt::Function { params, body, .. } => {
                for p in params {
                    self.check_binding(s.span.clone(), p);
                }
                self.visit_block(body);
            }
        }
    }

    fn visit_expr(&mut self, e: &Spanned<Expr>) {
        match &e.value {
            Expr::Call(c) => {
                self.check_call(e.span.clone(), c);
                for a in &c.args {
                    let inner = match &a.value {
                        CallArg::Positional(x) | CallArg::Named { value: x, .. } => x,
                    };
                    self.visit_expr(inner);
                }
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.visit_expr(lhs);
                self.visit_expr(rhs);
            }
            Expr::Unary { expr, .. } => self.visit_expr(expr),
            Expr::List(elems) => {
                for el in elems {
                    self.visit_expr(el);
                }
            }
            Expr::String(parts) => {
                for p in parts {
                    if let StringPart::Interpolation(inner) = p {
                        self.visit_expr(inner);
                    }
                }
            }
            Expr::Int(_) | Expr::Float(_) | Expr::Var(_) => {}
        }
    }

    /// A binding target (assign target, loop variable, parameter) must not use a
    /// name reserved for internal use: `checked` is the per-op verification flag,
    /// and `__`-prefixed names back hidden loop/return slots.
    fn check_binding(&mut self, span: Span, name: &str) {
        if is_reserved_binding(name) {
            self.diags.push(Diagnostic::error(
                span,
                format!("`{name}` is a reserved name and cannot be bound"),
            ));
        }
    }

    /// A call used as a value (assigned or returned). The formatter's `return`
    /// param determines whether the node produces a value and whether `return=`
    /// is required.
    fn check_value_expr(&mut self, e: &Spanned<Expr>) {
        let Expr::Call(c) = &e.value else {
            return;
        };
        // User-defined functions always return a value.
        if self.funcs.contains_key(&c.callee) {
            return;
        }
        let tmpl = resolve_template(&c.callee);
        match tmpl.return_spec().map(|p| p.kind) {
            Some(ParamKind::None) | None => {
                self.diags.push(Diagnostic::error(
                    e.span.clone(),
                    format!(
                        "`{}` produces no value and cannot be assigned or returned",
                        c.callee
                    ),
                ));
            }
            Some(ParamKind::String) if !has_named(c, "return") => {
                self.diags.push(Diagnostic::error(
                    e.span.clone(),
                    format!(
                        "value node `{}` used as a value needs a `return=` format argument",
                        c.callee
                    ),
                ));
            }
            // IntrinsicBool (judge) and List are assignable without return=.
            // String with a return= arg is also OK.
            _ => {}
        }
    }

    fn check_call(&mut self, span: Span, c: &Call) {
        // Duplicate named arguments (generic check, not formatter-driven).
        let mut seen = HashSet::new();
        for a in &c.args {
            if let CallArg::Named { name, .. } = &a.value {
                if !seen.insert(name.clone()) {
                    self.diags.push(Diagnostic::error(
                        a.span.clone(),
                        format!("duplicate argument `{name}`"),
                    ));
                }
            }
        }
        // User-defined functions: skip formatter checks (their param binding is
        // handled by the Call instruction, not by templates).
        if self.funcs.contains_key(&c.callee) {
            return;
        }
        // Formatter-driven param checks.
        let tmpl = resolve_template(&c.callee);
        // Check required `instruction` (first positional) is present.
        let has_positional = c
            .args
            .iter()
            .any(|a| matches!(a.value, CallArg::Positional(_)));
        if tmpl
            .params
            .iter()
            .any(|p| p.name == "instruction" && p.required)
            && !has_positional
        {
            self.diags.push(Diagnostic::error(
                span.clone(),
                format!("`{}` requires an instruction argument", c.callee),
            ));
        }
        // Type-check each named arg against the formatter.
        for a in &c.args {
            if let CallArg::Named { name, value } = &a.value {
                if let Some(spec) = tmpl
                    .params
                    .iter()
                    .find(|p| p.name.as_str() == name.as_str())
                {
                    // Skip meta-params (return: none / return: bool are not DSL args).
                    if matches!(spec.kind, ParamKind::None | ParamKind::IntrinsicBool) {
                        continue;
                    }
                    let ok = match (spec.kind, &value.value) {
                        (ParamKind::String, Expr::String(_)) => true,
                        (ParamKind::Bool, Expr::Var(v)) if v == "true" || v == "false" => true,
                        (ParamKind::List, Expr::List(_)) => true,
                        _ => false,
                    };
                    if !ok {
                        self.diags.push(Diagnostic::error(
                            a.span.clone(),
                            format!(
                                "argument `{name}` of `{}` must be a {}",
                                c.callee,
                                type_name(spec.kind)
                            ),
                        ));
                    }
                }
                // Unknown named args are silently accepted (forward-compatible).
            }
        }
        // Check required named params are present.
        for spec in &tmpl.params {
            if spec.name == "instruction" {
                continue;
            }
            if spec.required && !has_named(c, &spec.name) {
                self.diags.push(Diagnostic::error(
                    span.clone(),
                    format!("`{}` requires a `{}` argument", c.callee, spec.name),
                ));
            }
        }
    }
}

fn type_name(kind: ParamKind) -> &'static str {
    match kind {
        ParamKind::String => "string literal",
        ParamKind::Bool => "boolean (`true` or `false`)",
        ParamKind::List => "list literal",
        ParamKind::None | ParamKind::IntrinsicBool => "value",
    }
}

fn has_named(c: &Call, name: &str) -> bool {
    c.args
        .iter()
        .any(|a| matches!(&a.value, CallArg::Named { name: n, .. } if n == name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diags(src: &str) -> Vec<Diagnostic> {
        let m = steer_syntax::parse(src).expect("should parse");
        validate(&m)
    }

    fn assert_valid(src: &str) {
        let d = diags(src);
        assert!(d.is_empty(), "expected no diagnostics, got: {d:?}");
    }

    fn assert_one_diagnostic(src: &str, needle: &str) {
        let d = diags(src);
        assert_eq!(d.len(), 1, "expected exactly one diagnostic, got: {d:?}");
        assert!(
            d[0].message.contains(needle),
            "diagnostic `{}` does not mention `{needle}`",
            d[0].message
        );
    }

    #[test]
    fn valid_simple_workflow() {
        assert_valid("task(\"do something\")\n");
        assert_valid("x = task(\"do\", return=\"path\")\n");
        assert_valid("print(\"hi\")\n");
    }

    #[test]
    fn value_task_assigned_without_return_is_error() {
        assert_one_diagnostic("x = task(\"do\")\n", "return=");
        assert_one_diagnostic("y = ask(\"q\")\n", "return=");
        assert_one_diagnostic("z = command(\"ls\")\n", "return=");
        assert_one_diagnostic("w = collect(\"think\")\n", "return=");
    }

    #[test]
    fn value_task_assigned_with_return_is_ok() {
        assert_valid("x = task(\"do\", return=\"path\")\n");
        assert_valid("y = command(\"ls\", return=\"list\")\n");
    }

    #[test]
    fn judge_assigned_without_return_is_ok() {
        // `judge` returns a boolean intrinsically; no `return=` is required.
        assert_valid("passed = judge(\"is it done?\")\n");
        assert_valid("flag = judge(\"ready?\")\n");
    }

    #[test]
    fn reserved_binding_names_are_rejected() {
        // `checked` is the per-op flag; `__` prefixes hidden slots.
        assert_one_diagnostic("checked = 1\n", "reserved name");
        assert_one_diagnostic("__x = 1\n", "reserved name");
        assert_one_diagnostic("for __i in xs\n  print(__i)\nend\n", "reserved name");
        assert_one_diagnostic("func f(__p)\n  print(__p)\nend\n", "reserved name");
    }

    #[test]
    fn bare_task_without_return_is_ok() {
        // a task used purely for its side effects does not need return=
        assert_valid("task(\"do something\")\n");
        assert_valid("task(\"do\", check=\"ok\")\n");
    }

    #[test]
    fn print_cannot_be_assigned() {
        assert_one_diagnostic("x = print(\"hi\")\n", "no value");
    }

    #[test]
    fn return_of_value_task_needs_return_arg() {
        assert_one_diagnostic("func f()\n  return task(\"x\")\nend\n", "return=");
        assert_valid("func f()\n  return task(\"x\", return=\"p\")\nend\n");
    }

    #[test]
    fn user_func_assigned_is_ok_without_return_arg() {
        assert_valid("func helper()\n  return task(\"x\", return=\"p\")\nend\nr = helper()\n");
    }

    #[test]
    fn produce_must_be_list() {
        assert_one_diagnostic("task(\"x\", produce=\"not a list\")\n", "must be a list");
        assert_valid("task(\"x\", produce=[\"a\", \"b\"])\n");
    }

    #[test]
    fn check_must_be_string() {
        assert_one_diagnostic("task(\"x\", check=42)\n", "must be a string");
        assert_valid("task(\"x\", check=\"ok\")\n");
    }

    #[test]
    fn return_arg_must_be_string() {
        assert_one_diagnostic("x = task(\"d\", return=42)\n", "must be a string");
    }

    #[test]
    fn duplicate_named_argument() {
        assert_one_diagnostic(
            "task(\"x\", return=\"a\", return=\"b\")\n",
            "duplicate argument",
        );
    }

    #[test]
    fn duplicate_function() {
        assert_one_diagnostic(
            "func f()\n  print(\"a\")\nend\nfunc f()\n  print(\"b\")\nend\n",
            "duplicate function",
        );
    }

    #[test]
    fn duplicate_parameter() {
        assert_one_diagnostic("func f(a, a)\n  print(a)\nend\n", "duplicate parameter");
    }

    #[test]
    fn nested_blocks_are_checked() {
        // value task without return inside a loop body is still caught
        assert_one_diagnostic("for f in files\n  x = task(\"do\")\nend\n", "return=");
    }

    #[test]
    fn multiple_diagnostics_reported() {
        let d = diags("x = task(\"a\")\ny = print(\"b\")\n");
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn root_cause_bugfix_workflow_is_valid() {
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
"#;
        assert_valid(src);
    }
}
