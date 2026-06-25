//! The instruction-stepping interpreter over the IR.
//!
//! [`step`] advances the program counter past control instructions, pausing at
//! the next agent op and returning its rendered instruction. [`check`] advances
//! past an agent op once the agent has reported (via `set` / `checked`). The
//! interpreter holds no state of its own — everything lives in the [`Context`],
//! so a run resumes transparently between calls.

use std::collections::HashMap;

use steer_syntax::ast::{Call, CallArg};

use crate::context::{CheckedReport, Context, Frame, Status, WorkflowMeta};
use crate::ir::Instr;
use crate::template::{render_call, resolve_template_with_meta, ParamKind};
use crate::value::{eval, EvalError, Value};

/// Outcome of [`step`].
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    /// Paused at an agent op; the rendered instruction for the agent.
    Instruction(String),
    /// The run finished via `Halt` or a top-level `return`.
    Complete,
    /// The run was already not running.
    NotRunning,
    /// A control expression could not be evaluated.
    Error(EvalError),
}

/// Outcome of [`check`].
#[derive(Debug, Clone, PartialEq)]
pub enum CheckOutcome {
    /// The current op passed; the program counter advanced.
    Advanced,
    /// Verification instructions for the current op.
    Instruction(String),
    /// The op's value/check has not been reported yet.
    Pending,
    /// The op's check failed; the program counter stays for a retry.
    Failed,
    /// There was no current agent op, or the program ended.
    Done,
    /// The run was already not running.
    NotRunning,
    /// A control expression could not be evaluated.
    Error(EvalError),
}

/// Record an evaluation error and halt the run: the PC sits on the failing
/// instruction, so the run cannot make progress. The error is still returned
/// so the caller can report it.
fn eval_error(ctx: &mut Context, e: EvalError) -> StepOutcome {
    ctx.status = Status::Halted(e.to_string());
    StepOutcome::Error(e)
}

/// Advance past control instructions, pausing at the next agent op.
///
/// `instance` is the run's instance name, threaded into the rendered
/// instruction so the agent sees the exact `steer instance set <name> ...`
/// command to report back.
pub fn step(ir: &[Instr], ctx: &mut Context, instance: &str) -> StepOutcome {
    if !ctx.is_running() {
        return StepOutcome::NotRunning;
    }
    loop {
        let Some(instr) = ir.get(ctx.pc as usize) else {
            ctx.status = Status::Complete;
            return StepOutcome::Complete;
        };
        match instr {
            Instr::Halt => {
                ctx.status = Status::Complete;
                return StepOutcome::Complete;
            }
            Instr::AgentOp { call, into } => {
                let mut text =
                    render_call(call, into.as_deref(), Some(&ctx.vars), &ctx.meta, instance);
                if let Some(step_state) = ctx.steps.get(&ctx.pc) {
                    if let Some(reason) = step_state.failure_reason.as_deref() {
                        text = append_retry_context(text, reason, step_state.retry_count);
                    }
                }
                return StepOutcome::Instruction(text);
            }
            Instr::SetMeta { key, expr } => match eval(expr, &ctx.vars) {
                Ok(v) => {
                    apply_meta(ctx, key, v);
                    ctx.pc += 1;
                }
                Err(e) => return eval_error(ctx, e),
            },
            Instr::Assign { var, expr } => match eval(expr, &ctx.vars) {
                Ok(v) => {
                    ctx.vars.insert(var.clone(), v);
                    ctx.pc += 1;
                }
                Err(e) => return eval_error(ctx, e),
            },
            Instr::JumpIfFalse { cond, target } => match eval(cond, &ctx.vars) {
                Ok(v) => {
                    if !v.truthy() {
                        ctx.pc = *target;
                    } else {
                        ctx.pc += 1;
                    }
                }
                Err(e) => return eval_error(ctx, e),
            },
            Instr::Jump { target } => ctx.pc = *target,
            Instr::ForInit { iter, list } => match eval(list, &ctx.vars) {
                Ok(v) => {
                    ctx.vars.insert(iter.clone(), v);
                    ctx.pc += 1;
                }
                Err(e) => return eval_error(ctx, e),
            },
            Instr::ForIter { iter, var, end } => {
                let next = match ctx.vars.get(iter) {
                    Some(Value::List(items)) if !items.is_empty() => Some(items[0].clone()),
                    _ => None,
                };
                match next {
                    Some(first) => {
                        let rest = match ctx.vars.get(iter) {
                            Some(Value::List(items)) => items[1..].to_vec(),
                            _ => Vec::new(),
                        };
                        ctx.vars.insert(var.clone(), first);
                        ctx.vars.insert(iter.clone(), Value::List(rest));
                        ctx.pc += 1;
                    }
                    None => ctx.pc = *end,
                }
            }
            Instr::Call {
                entry,
                params,
                args,
                into,
            } => {
                let mut scope = HashMap::new();
                for (p, a) in params.iter().zip(args.iter()) {
                    match eval(a, &ctx.vars) {
                        Ok(v) => {
                            scope.insert(p.clone(), v);
                        }
                        Err(e) => return eval_error(ctx, e),
                    }
                }
                let saved = std::mem::take(&mut ctx.vars);
                ctx.vars = scope;
                ctx.frames.push(Frame {
                    return_pc: ctx.pc + 1,
                    into: into.clone(),
                    saved_vars: saved,
                });
                ctx.pc = *entry;
            }
            Instr::Return { value } => {
                let rv = match value {
                    Some(e) => eval(e, &ctx.vars),
                    None => Ok(Value::Null),
                };
                match rv {
                    Ok(v) => match ctx.frames.pop() {
                        Some(frame) => {
                            ctx.vars = frame.saved_vars;
                            if let Some(t) = frame.into {
                                ctx.vars.insert(t, v);
                            }
                            ctx.pc = frame.return_pc;
                        }
                        None => {
                            ctx.status = Status::Complete;
                            return StepOutcome::Complete;
                        }
                    },
                    Err(e) => return eval_error(ctx, e),
                }
            }
        }
    }
}

fn apply_meta(ctx: &mut Context, key: &str, value: Value) {
    if key == "template" {
        let rendered = value.render();
        ctx.meta.template_dir = if rendered.is_empty() {
            None
        } else {
            Some(rendered)
        };
    } else if key == "context" {
        let rendered = value.render();
        ctx.meta.context = if rendered.is_empty() {
            None
        } else {
            Some(rendered)
        };
    }
}

/// Advance past the current agent op, dispatching on how it is verified.
///
/// `instance` is threaded into the verification instruction so the agent sees
/// the exact `steer instance set <name> checked ...` command.
pub fn check(ir: &[Instr], ctx: &mut Context, instance: &str) -> CheckOutcome {
    if !ctx.is_running() {
        return CheckOutcome::NotRunning;
    }
    let Some(instr) = ir.get(ctx.pc as usize) else {
        return CheckOutcome::Done;
    };
    let Instr::AgentOp { call, into } = instr else {
        return CheckOutcome::Done;
    };
    match check_kind(call, into.as_deref()) {
        CheckKind::Auto => {
            ctx.pc += 1;
            CheckOutcome::Advanced
        }
        CheckKind::Value(target) => {
            if ctx.vars.contains_key(&target) {
                ctx.pc += 1;
                CheckOutcome::Advanced
            } else {
                CheckOutcome::Pending
            }
        }
        CheckKind::Checked => {
            let pc = ctx.pc;
            let checked = ctx.steps.get(&pc).and_then(|st| st.checked.clone());
            match checked {
                Some(report) if report.passed() => {
                    // Consume the op's verification state. Inside a loop the
                    // same `AgentOp` instruction is re-entered each iteration;
                    // leaving a prior pass in place would let a later iteration
                    // advance without a fresh report. Clearing here (rather than
                    // relying on PC advancement) makes "a pass is reported once
                    // and consumed once" hold uniformly, loop or not. The
                    // failure path below keeps its report for retry.
                    ctx.steps.remove(&pc);
                    ctx.pc += 1;
                    CheckOutcome::Advanced
                }
                Some(report) => {
                    let reason = report
                        .failure_reason()
                        .unwrap_or("Verification failed without a reason.")
                        .to_string();
                    let st = ctx.steps.entry(pc).or_default();
                    // Keep `checked` as-is for audit; store failure reason
                    // for retry context. Increment retry count.
                    st.failure_reason = Some(reason);
                    st.retry_count += 1;
                    CheckOutcome::Failed
                }
                None => {
                    match check_instruction(call, into.as_deref(), &ctx.vars, &ctx.meta, instance) {
                        Ok(instruction) => CheckOutcome::Instruction(instruction),
                        Err(e) => CheckOutcome::Error(e),
                    }
                }
            }
        }
    }
}

enum CheckKind {
    Auto,
    Value(String),
    Checked,
}

fn check_kind(call: &Call, into: Option<&str>) -> CheckKind {
    let has_check = call
        .args
        .iter()
        .any(|a| matches!(&a.value, CallArg::Named { name, .. } if name == "check"));
    if has_check {
        CheckKind::Checked
    } else if let Some(t) = into {
        CheckKind::Value(t.to_string())
    } else {
        CheckKind::Auto
    }
}

fn check_instruction(
    call: &Call,
    into: Option<&str>,
    vars: &HashMap<String, Value>,
    meta: &WorkflowMeta,
    instance: &str,
) -> Result<String, EvalError> {
    let tmpl = crate::template::resolve_template_with_meta(&call.callee, meta);
    let instruction = crate::template::render_check(&tmpl, call, into, Some(vars), instance)?;
    Ok(format!(
        "{instruction}\n\n<steer-system-reminder>\n{}\n</steer-system-reminder>",
        crate::template::render_check_report(instance)
    ))
}

/// Validate a value the agent is about to `set` against the current op's
/// declared `return` type. This is the engine's last line of defense against an
/// agent that — e.g. after context compression — forgets the exact `steer
/// instance set` command and stores a wrong-typed value (such as a JSON verdict
/// object, or `false`, into a string variable; or a non-bool into a `judge`
/// bool variable).
///
/// Enforcement happens at `set` time, not `check` time, because a value op has
/// no `check=` gate forcing the agent to call `check` — the agent could `set`
/// and then `step` past the op without ever checking. Validating at `set` means
/// a wrong-typed value is rejected immediately and never stored.
///
/// Only the current op's assignment target is type-checked: `var` must equal
/// `into` of `ir[ctx.pc]` when that instruction is an `AgentOp`. Setting any
/// other variable (a workflow-assigned local, an already-completed op's
/// variable) is not constrained by a callee return type and is allowed.
///
/// Returns `Ok(())` when the value matches, when the op declares no type to
/// enforce (`None`, missing `return` spec, bare call), or when `var` is not the
/// current op's target; returns `Err(reason)` with a kind-specific reason on
/// mismatch.
pub fn validate_set_value(
    ir: &[Instr],
    ctx: &Context,
    var: &str,
    value: &Value,
) -> Result<(), String> {
    let Some(Instr::AgentOp { call, into }) = ir.get(ctx.pc as usize) else {
        return Ok(());
    };
    let Some(target) = into.as_deref() else {
        return Ok(());
    };
    if target != var {
        return Ok(());
    }
    check_value_against_callee(call, value, &ctx.meta)
}

/// Check a value against a callee's declared `return` type. See
/// [`validate_set_value`] for the policy that decides when this runs.
fn check_value_against_callee(
    call: &Call,
    value: &Value,
    meta: &WorkflowMeta,
) -> Result<(), String> {
    let kind = resolve_template_with_meta(&call.callee, meta)
        .return_spec()
        .map(|spec| spec.kind);
    match kind {
        Some(ParamKind::IntrinsicBool) | Some(ParamKind::Bool) => match value {
            Value::Bool(_) => Ok(()),
            other => Err(format!(
                "expected a boolean (true/false) for `{}`, got {}",
                call.callee,
                value_kind_name(other)
            )),
        },
        Some(ParamKind::String) => match value {
            Value::Str(_) => Ok(()),
            other => Err(format!(
                "expected a string for `{}`, got {} — if you meant to report structured data, that's not supported by return:string",
                call.callee,
                value_kind_name(other)
            )),
        },
        // `None`, a missing `return` spec, or any other kind: no declared
        // type to enforce.
        _ => Ok(()),
    }
}

/// A short human-readable name for a `Value` variant, for failure reasons.
fn value_kind_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Int(_) | Value::Float(_) => "number",
        Value::Str(_) => "string",
        Value::List(_) => "list",
        Value::Object(_) => "object",
    }
}

fn append_retry_context(instruction: String, reason: &str, retry_count: u32) -> String {
    format!(
        "{instruction}\n\n<steer-system-reminder>\n{}\n</steer-system-reminder>",
        crate::template::render_retry_context(reason, retry_count)
    )
}

/// Write a value into the context: a normal variable, or the special `checked`
/// flag for the current agent op.
pub fn set_value(ctx: &mut Context, var: &str, value: Value) -> Result<(), String> {
    if var == "checked" {
        let report = checked_report(value)?;
        ctx.steps.entry(ctx.pc).or_default().checked = Some(report);
    } else {
        ctx.vars.insert(var.to_string(), value);
    }
    Ok(())
}

fn checked_report(value: Value) -> Result<CheckedReport, String> {
    match value {
        Value::Bool(true) => Ok(CheckedReport::Bool(true)),
        Value::Bool(false) => Err(
            "`checked=false` is invalid; use `{\"passed\":false,\"reason\":\"...\"}`".to_string(),
        ),
        Value::Object(mut object) => {
            let passed = match object.remove("passed") {
                Some(Value::Bool(passed)) => passed,
                _ => return Err("`checked` object must include boolean `passed`".to_string()),
            };
            let reason = match object.remove("reason") {
                Some(Value::Str(reason)) if !reason.trim().is_empty() => Some(reason),
                Some(Value::Str(_)) if !passed => {
                    return Err("failed `checked` object must include a non-empty `reason`".into());
                }
                Some(_) => return Err("`checked.reason` must be a string".to_string()),
                None if !passed => {
                    return Err("failed `checked` object must include `reason`".to_string());
                }
                None => None,
            };
            Ok(CheckedReport::Object { passed, reason })
        }
        _ => Err("`checked` must be true or an object with `passed` and `reason`".to_string()),
    }
}

/// Record a fatal error and halt the run.
pub fn report_error(ctx: &mut Context, reason: &str) {
    ctx.status = Status::Halted(reason.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::lower;

    fn ir(src: &str) -> Vec<Instr> {
        lower(&steer_syntax::parse(src).expect("parses"))
    }

    #[test]
    fn step_stops_at_first_agent_op() {
        let ir = ir("print(\"hi\")\n");
        let mut ctx = Context::new();
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("hi"))
        );
        assert_eq!(ctx.pc, 0); // step does not advance at an agent op
    }

    #[test]
    fn check_auto_advances_past_print() {
        let ir = ir("print(\"a\")\nprint(\"b\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>");
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("b"))
        );
        check(&ir, &mut ctx, "<name>");
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
    }

    #[test]
    fn assignment_runs_before_step_pausing() {
        let ir = ir("x = 5\nprint(x)\n");
        let mut ctx = Context::new();
        // first step runs the assignment then pauses at print, with x resolved
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(s.contains("5"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
    }

    #[test]
    fn value_op_check_pending_until_set() {
        let ir = ir("x = ask(\"q\", return=\"str\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at ask
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Pending);
        set_value(&mut ctx, "x", Value::Str("answer".into())).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
    }

    // ---- return-type enforcement on set ----

    #[test]
    fn bool_return_accepts_boolean_at_set() {
        let ir = ir("covered = judge(\"is it covered?\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at judge
        for v in [Value::Bool(true), Value::Bool(false)] {
            assert!(
                validate_set_value(&ir, &ctx, "covered", &v).is_ok(),
                "bool value should be accepted"
            );
        }
    }

    #[test]
    fn bool_return_rejects_non_boolean_at_set() {
        // The reported bug: agent sets a JSON verdict object into `covered`
        // instead of a bool. set must reject it before storing.
        let ir = ir("covered = judge(\"is it covered?\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at judge
        let verdict = Value::Object(HashMap::from([
            ("verdict".to_string(), Value::Str("COVERED".into())),
            ("prior_gap".to_string(), Value::Int(354)),
        ]));
        let err = validate_set_value(&ir, &ctx, "covered", &verdict).unwrap_err();
        assert!(err.contains("boolean"), "reason: {err}");
        // Rejection happens before set_value, so nothing is stored.
        assert!(!ctx.vars.contains_key("covered"));
    }

    #[test]
    fn string_return_accepts_string_at_set() {
        let ir = ir("x = ask(\"q\", return=\"str\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at ask
        assert!(validate_set_value(&ir, &ctx, "x", &Value::Str("answer".into())).is_ok());
    }

    #[test]
    fn string_return_rejects_non_string_at_set() {
        // The user's reproduced case: `bug_slug = ask(..., return="...")` is a
        // string return, but the agent sets `false` (a bool). set must reject.
        let ir = ir("bug_slug = ask(\"q\", return=\"bug slug in kebab-case\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at ask
        let err = validate_set_value(&ir, &ctx, "bug_slug", &Value::Bool(false)).unwrap_err();
        assert!(
            err.contains("string") && err.contains("return:string"),
            "reason: {err}"
        );
        assert!(!ctx.vars.contains_key("bug_slug"));
    }

    #[test]
    fn string_return_rejects_object_at_set() {
        let ir = ir("x = ask(\"q\", return=\"str\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at ask
        let obj = Value::Object(HashMap::from([(
            "verdict".to_string(),
            Value::Str("COVERED".into()),
        )]));
        let err = validate_set_value(&ir, &ctx, "x", &obj).unwrap_err();
        assert!(
            err.contains("string") && err.contains("return:string"),
            "reason: {err}"
        );
    }

    #[test]
    fn set_of_non_current_op_var_is_not_type_checked() {
        // Setting a variable that is NOT the current op's target (e.g. a
        // workflow-local, or an already-completed op's variable) must not be
        // constrained by any callee return type.
        let ir = ir("x = ask(\"q\", return=\"str\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at ask (target is `x`)
                                       // `other` is not the current op's target — any value is allowed.
        assert!(validate_set_value(&ir, &ctx, "other", &Value::Bool(false)).is_ok());
        assert!(validate_set_value(&ir, &ctx, "other", &Value::Int(7)).is_ok(),);
    }

    #[test]
    fn unchecked_return_kinds_pass_at_set() {
        // A bare custom callee (no return spec -> generic fallback) assigned to
        // a var: any value is accepted — no declared type to enforce.
        let ir = ir("x = mycaller(\"do\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>"); // pause at mycaller
        let obj = Value::Object(HashMap::from([("k".to_string(), Value::Str("v".into()))]));
        assert!(validate_set_value(&ir, &ctx, "x", &obj).is_ok());
        assert!(validate_set_value(&ir, &ctx, "x", &Value::Bool(false)).is_ok());
    }

    #[test]
    fn task_with_check_uses_checked_flag() {
        let ir = ir("task(\"do\", check=\"ok\")\n");
        let mut ctx = Context::new();
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(!s.contains("ok"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
        match check(&ir, &mut ctx, "<name>") {
            CheckOutcome::Instruction(s) => {
                assert!(s.contains("ok"), "got: {s}");
                assert!(s.contains("steer instance set <name> checked"), "got: {s}");
            }
            o => panic!("unexpected {o:?}"),
        }
        assert!(set_value(&mut ctx, "checked", Value::Bool(false)).is_err());
        let failed = Value::Object(HashMap::from([
            ("passed".to_string(), Value::Bool(false)),
            ("reason".to_string(), Value::Str("tests failed".into())),
        ]));
        set_value(&mut ctx, "checked", failed).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Failed);
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => {
                assert!(
                    s.contains("Previous verification failed (retry #1)"),
                    "got: {s}"
                );
                assert!(s.contains("tests failed"), "got: {s}");
            }
            o => panic!("unexpected {o:?}"),
        }
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
    }

    #[test]
    fn if_branch_taken_at_runtime() {
        let ir = ir("if 1 > 0\n  print(\"yes\")\nelse\n  print(\"no\")\nend\n");
        let mut ctx = Context::new();
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(s.contains("yes"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
    }

    #[test]
    fn elseif_runs_the_first_matching_branch() {
        // x == 2 matches the `elseif`, so only `print("two")` should run.
        let ir = ir(
            "x = 2\nif x == 1\n  print(\"one\")\nelseif x == 2\n  print(\"two\")\nelse\n  print(\"other\")\nend\n",
        );
        let mut ctx = Context::new();
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(s.contains("two"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
    }

    #[test]
    fn func_call_resolves_return_value() {
        let ir = ir("func g()\n  return \"r\"\nend\nx = g()\nprint(x)\n");
        let mut ctx = Context::new();
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(s.contains("r"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
    }

    #[test]
    fn for_loop_iterations() {
        // iterate a literal list; the body runs once per element
        let ir = ir("for f in [\"a\", \"b\"]\n  print(f)\nend\n");
        let mut ctx = Context::new();
        // iteration 1
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("a"))
        );
        check(&ir, &mut ctx, "<name>");
        // iteration 2
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("b"))
        );
        check(&ir, &mut ctx, "<name>");
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
    }

    #[test]
    fn for_loop_check_requires_per_iteration_report() {
        // A check-bearing task inside a for loop lowers to ONE AgentOp that each
        // iteration re-enters. A pass reported for the first iteration must NOT
        // satisfy the second iteration's check.
        let ir = ir("for f in [\"a\", \"b\"]\n  task(\"fix {f}\", check=\"confirm {f}\")\nend\n");
        let mut ctx = Context::new();
        // iteration 1: "a"
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("fix a"))
        );
        // not reported yet -> verification instruction
        assert!(matches!(
            check(&ir, &mut ctx, "<name>"),
            CheckOutcome::Instruction(_)
        ));
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        // iteration 2: "b". The stale pass for "a" must be gone.
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("fix b"))
        );
        // Bug: this reads Advanced on the stale "a" pass. Expected: Instruction.
        assert!(
            matches!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Instruction(_)),
            "second iteration must require a fresh report"
        );
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
    }

    #[test]
    fn loop_until_check_requires_per_iteration_report() {
        // A check-bearing task inside loop...until re-enters the same AgentOp each
        // iteration; a prior pass must not leak across the back-edge.
        let ir = ir("i = 0\nloop\n  i = i + 1\n  task(\"step {i}\", check=\"ok\")\nuntil i >= 2\n");
        let mut ctx = Context::new();
        // iteration 1: "step 1"
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("step 1"))
        );
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        // iteration 2: "step 2" — stale pass must be gone.
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("step 2"))
        );
        assert!(
            matches!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Instruction(_)),
            "second iteration must require a fresh report"
        );
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
    }

    #[test]
    fn passing_check_consumes_report_single_iteration() {
        let ir = ir("task(\"do\", check=\"ok\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>");
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        // The report is consumed; the op is no longer pending any report.
        assert!(ctx.steps.get(&0).is_none_or(|s| s.checked.is_none()));
        // A second check on the now-advanced pc lands beyond the AgentOp.
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Done);
    }

    #[test]
    fn failed_then_pass_within_iteration_consumes_report() {
        let ir = ir("for f in [\"a\", \"b\"]\n  task(\"fix {f}\", check=\"confirm {f}\")\nend\n");
        let mut ctx = Context::new();
        // iteration 1: fail, then pass on retry within the same iteration.
        assert!(matches!(
            step(&ir, &mut ctx, "<name>"),
            StepOutcome::Instruction(_)
        ));
        let failed = Value::Object(HashMap::from([
            ("passed".to_string(), Value::Bool(false)),
            ("reason".to_string(), Value::Str("nope".into())),
        ]));
        set_value(&mut ctx, "checked", failed).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Failed);
        // retry step surfaces the failure reason
        assert!(
            matches!(step(&ir, &mut ctx, "<name>"), StepOutcome::Instruction(s) if s.contains("nope"))
        );
        set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
        assert_eq!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Advanced);
        // iteration 2: the consumed pass for iteration 1 must not leak.
        assert!(matches!(
            step(&ir, &mut ctx, "<name>"),
            StepOutcome::Instruction(_)
        ));
        assert!(
            matches!(check(&ir, &mut ctx, "<name>"), CheckOutcome::Instruction(_)),
            "second iteration must require a fresh report after a failed-then-pass"
        );
    }

    #[test]
    fn report_error_halts() {
        let ir = ir("print(\"a\")\n");
        let mut ctx = Context::new();
        step(&ir, &mut ctx, "<name>");
        report_error(&mut ctx, "boom");
        assert!(!ctx.is_running());
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::NotRunning);
    }

    #[test]
    fn full_example_bugfix_loop_run_with_mock_agent() {
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
        let ir = ir(src);
        let mut ctx = Context::new();
        // Drive the whole workflow: for each instruction, mock the agent by
        // supplying a value for value-ops and auto-passing checks.
        let mut guard = 0;
        while ctx.is_running() && guard < 100 {
            guard += 1;
            match step(&ir, &mut ctx, "<name>") {
                StepOutcome::Instruction(_) => {
                    // mock the agent's response based on how the op is checked
                    match check(&ir, &mut ctx, "<name>") {
                        CheckOutcome::Pending => {
                            // A value-op awaiting its value.
                            if let Some(Instr::AgentOp { into, call }) = ir.get(ctx.pc as usize) {
                                if into.is_some()
                                    && !call.args.iter().any(|a| {
                                        matches!(&a.value, CallArg::Named { name, .. } if name == "check")
                                    })
                                {
                                    set_value(&mut ctx, into.as_ref().unwrap(), Value::Str("v".into())).unwrap();
                                }
                            }
                        }
                        CheckOutcome::Instruction(_) => {
                            set_value(&mut ctx, "checked", Value::Bool(true)).unwrap();
                        }
                        CheckOutcome::Failed | CheckOutcome::Advanced | CheckOutcome::Done => {}
                        CheckOutcome::NotRunning | CheckOutcome::Error(_) => break,
                    }
                }
                StepOutcome::Complete | StepOutcome::NotRunning => break,
                StepOutcome::Error(e) => panic!("eval error: {e}"),
            }
        }
        assert_eq!(ctx.status, Status::Complete);
    }
}
