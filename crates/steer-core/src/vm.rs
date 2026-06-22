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
use crate::template::render_call;
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
                if let Some(reason) = ctx
                    .steps
                    .get(&ctx.pc)
                    .and_then(|state| state.failure_reason.as_deref())
                {
                    text = append_retry_context(text, reason);
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
                    // Keep `checked` as-is so the verification result persists
                    // in context.json as an audit trail. PC advancement
                    // already prevents re-checking this step.
                    if let Some(st) = ctx.steps.get_mut(&pc) {
                        st.failure_reason = None;
                    }
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
                    // for retry context.
                    st.failure_reason = Some(reason);
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
        "{instruction}\n\nReport the verification result:\n- Passed: `steer instance set {instance} checked {{\"passed\":true}}`\n- Failed: `steer instance set {instance} checked {{\"passed\":false,\"reason\":\"<why it failed>\"}}`"
    ))
}

fn append_retry_context(mut instruction: String, reason: &str) -> String {
    instruction.push_str("\n\nPrevious verification failed:\n");
    instruction.push_str(reason);
    instruction.push_str("\n\nRetry the task and address the failure before checking again.");
    instruction
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
                assert!(s.contains("Previous verification failed"), "got: {s}");
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
        let ir = ir(include_str!(
            "../../../examples/workflows/bugfix-loop.steer"
        ));
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
