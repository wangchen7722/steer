//! Static dry-run of a workflow: render every action-node instruction in source
//! order.
//!
//! This is a *static* walk — it does not execute control flow, resolve
//! variables, or inline function calls. Loops are shown once (not expanded),
//! both branches of an `if` are shown, and a function's instructions are shown
//! where the function is *defined* (a call to a user function is not itself an
//! instruction). Runtime `{var}` interpolations are kept as placeholders.
//!
//! The full mock-walk (with variable resolution, function inlining and loop
//! expansion) arrives with the real interpreter in a later milestone.

use std::collections::HashSet;

use steer_syntax::ast::{Call, CallArg, Expr, Stmt};
use steer_syntax::{Module, Spanned};

use crate::context::WorkflowMeta;
use crate::template::render_call;
use crate::value::eval_literal;

/// One rendered instruction in the dry-run output.
#[derive(Debug, Clone, PartialEq)]
pub struct SimStep {
    /// The callee name, e.g. `task`, `ask`, or `command`.
    pub callee: String,
    /// The rendered instruction text.
    pub instruction: String,
}

/// Render every action-node instruction in `module`, in source order.
pub fn simulate(module: &Module) -> Vec<SimStep> {
    let mut funcs = HashSet::new();
    collect_funcs(&module.body, &mut funcs);
    let mut out = Vec::new();
    let mut meta = WorkflowMeta::default();
    walk_block(&module.body, &funcs, &mut meta, &mut out);
    out
}

fn collect_funcs(block: &[Spanned<Stmt>], funcs: &mut HashSet<String>) {
    for s in block {
        match &s.value {
            Stmt::Function { name, body, .. } => {
                funcs.insert(name.clone());
                collect_funcs(body, funcs);
            }
            Stmt::If {
                branches,
                else_block,
            } => {
                for b in branches {
                    collect_funcs(&b.body, funcs);
                }
                if let Some(eb) = else_block {
                    collect_funcs(eb, funcs);
                }
            }
            Stmt::LoopUntil { body, .. } | Stmt::For { body, .. } => {
                collect_funcs(body, funcs);
            }
            _ => {}
        }
    }
}

fn walk_block(
    block: &[Spanned<Stmt>],
    funcs: &HashSet<String>,
    meta: &mut WorkflowMeta,
    out: &mut Vec<SimStep>,
) {
    for s in block {
        walk_stmt(s, funcs, meta, out);
    }
}

fn walk_stmt(
    s: &Spanned<Stmt>,
    funcs: &HashSet<String>,
    meta: &mut WorkflowMeta,
    out: &mut Vec<SimStep>,
) {
    match &s.value {
        Stmt::Meta { key, value } => apply_static_meta(meta, key, value),
        Stmt::Call(call) => render_call_node(call, None, funcs, meta, out),
        Stmt::Assign { target, value } => render_if_call(value, Some(target), funcs, meta, out),
        Stmt::Return { value } => {
            if let Some(v) = value {
                render_if_call(v, None, funcs, meta, out);
            }
        }
        Stmt::If {
            branches,
            else_block,
        } => {
            for b in branches {
                render_if_call(&b.cond, None, funcs, meta, out);
                walk_block(&b.body, funcs, meta, out);
            }
            if let Some(eb) = else_block {
                walk_block(eb, funcs, meta, out);
            }
        }
        Stmt::LoopUntil { body, cond } => {
            walk_block(body, funcs, meta, out);
            render_if_call(cond, None, funcs, meta, out);
        }
        Stmt::For { iterable, body, .. } => {
            render_if_call(iterable, None, funcs, meta, out);
            walk_block(body, funcs, meta, out);
        }
        Stmt::Function { body, .. } => walk_block(body, funcs, meta, out),
    }
}

fn apply_static_meta(meta: &mut WorkflowMeta, key: &str, value: &Spanned<Expr>) {
    let value = eval_literal(value);
    if key == "template" {
        let rendered = value.render();
        meta.template_dir = if rendered.is_empty() {
            None
        } else {
            Some(rendered)
        };
    } else if key == "context" {
        let rendered = value.render();
        meta.context = if rendered.is_empty() {
            None
        } else {
            Some(rendered)
        };
    }
}

/// If `call` is to an action node rather than a user function, render and push it.
fn render_call_node(
    call: &Call,
    into: Option<&str>,
    funcs: &HashSet<String>,
    meta: &WorkflowMeta,
    out: &mut Vec<SimStep>,
) {
    if !funcs.contains(&call.callee) {
        out.push(SimStep {
            callee: call.callee.clone(),
            instruction: render_call(call, into, None, meta, "<name>"),
        });
    }
}

/// Recursively render every action-node call inside `e` — including calls
/// nested in binary/unary operations, list literals, string interpolations,
/// and call arguments.
fn render_if_call(
    e: &Spanned<Expr>,
    into: Option<&str>,
    funcs: &HashSet<String>,
    meta: &mut WorkflowMeta,
    out: &mut Vec<SimStep>,
) {
    match &e.value {
        Expr::Call(c) => {
            render_call_node(c, into, funcs, meta, out);
            for a in &c.args {
                let inner = match &a.value {
                    CallArg::Positional(x) | CallArg::Named { value: x, .. } => x,
                };
                render_if_call(inner, None, funcs, meta, out);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            render_if_call(lhs, None, funcs, meta, out);
            render_if_call(rhs, None, funcs, meta, out);
        }
        Expr::Unary { expr, .. } => render_if_call(expr, None, funcs, meta, out),
        Expr::List(elems) => {
            for el in elems {
                render_if_call(el, None, funcs, meta, out);
            }
        }
        // Int, Float, Var, String carry no nested calls (string interpolations
        // inside an interpolation body are rejected by the lexer).
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulate_src(src: &str) -> Vec<SimStep> {
        let m = steer_syntax::parse(src).expect("parses");
        simulate(&m)
    }

    fn callees(steps: &[SimStep]) -> Vec<&str> {
        steps.iter().map(|s| s.callee.as_str()).collect()
    }

    #[test]
    fn renders_single_task() {
        let steps = simulate_src("task(\"do something\")\n");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].callee, "task");
        assert!(steps[0].instruction.contains("do something"));
    }

    #[test]
    fn renders_in_source_order() {
        let steps = simulate_src("task(\"a\")\nprint(\"b\")\nask(\"c\", return=\"x\")\n");
        assert_eq!(callees(&steps), vec!["task", "print", "ask"]);
    }

    #[test]
    fn assignment_call_simulates_set_prompt_with_target() {
        let steps = simulate_src("answer = ask(\"q\", return=\"str\")\n");
        assert_eq!(steps.len(), 1);
        assert!(
            steps[0].instruction.contains("steer instance set")
                && steps[0].instruction.contains("answer")
        );
    }

    #[test]
    fn skips_user_function_calls_but_renders_their_bodies() {
        // analyze is a user function: its call site is skipped, but the task
        // inside its body is rendered.
        let src = "func analyze()\n  task(\"inside func\")\nend\nrc = analyze()\n";
        let steps = simulate_src(src);
        assert_eq!(callees(&steps), vec!["task"]);
        assert!(steps[0].instruction.contains("inside func"));
    }

    #[test]
    fn renders_both_if_branches() {
        let src = "if x\n  task(\"then\")\nelse\n  task(\"else\")\nend\n";
        let steps = simulate_src(src);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn loop_body_shown_once() {
        let src = "for f in files\n  task(\"fix {f}\")\nend\n";
        let steps = simulate_src(src);
        assert_eq!(steps.len(), 1);
        assert!(steps[0].instruction.contains("fix {f}"));
    }

    #[test]
    fn judge_node_is_rendered() {
        let steps = simulate_src("passed = judge(\"ok?\")\n");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].callee, "judge");
        assert!(steps[0].instruction.contains("ok?"));
        assert!(steps[0].instruction.contains("true"));
    }

    #[test]
    fn iterable_call_is_rendered() {
        let src = "for f in command(\"ls\", return=\"list\")\n  print(f)\nend\n";
        let steps = simulate_src(src);
        assert_eq!(callees(&steps), vec!["command", "print"]);
    }

    #[test]
    fn empty_workflow_yields_nothing() {
        assert!(simulate_src("// just a comment\n").is_empty());
    }

    #[test]
    fn example_bugfix_loop_workflow_steps() {
        let src = r#"
bug = ask("Which bug?", return="bug id")
root_cause = collect("Reproduce {bug}", return="root cause", check="confirm summary")
files = command("git diff --name-only", return="files")
attempt = 0
passed = false
loop
    attempt = attempt + 1
    task("Fix {bug} attempt {attempt}", check="verify fix")
    passed = judge("Did the fix work?")
until passed or attempt >= 3
if not passed
    task("Write handoff", produce=["handoff.md"], check="confirm handoff")
    return
end
for f in files
    task("Review {f}", check="confirm {f} is clean")
end
print("done")
"#;
        let steps = simulate_src(src);
        assert_eq!(
            callees(&steps),
            vec!["ask", "collect", "command", "task", "judge", "task", "task", "print"]
        );
    }
}
