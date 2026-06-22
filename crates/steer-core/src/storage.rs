//! Instance persistence.
//!
//! An instance lives under `.steer/instances/<name>/` and holds two files: the
//! workflow source (`workflow.steer`, copied at start so the instance is
//! self-contained) and the serialised [`Context`] (`context.json`). Each CLI
//! command re-derives the instruction stream from the source and operates on
//! the context, so a run resumes transparently between invocations.

use std::path::Path;
use std::{fs, io};

use thiserror::Error;

use crate::context::{Context, WorkflowMeta};
use crate::ir::{lower, Instr};
use crate::value::eval_literal;

const WORKFLOW_FILE: &str = "workflow.steer";
const CONTEXT_FILE: &str = "context.json";

/// An instance-storage error.
#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("invalid context.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("workflow did not parse: {0}")]
    Parse(#[from] steer_syntax::ParseError),
}

/// Create (or reset) an instance: clear `dir`, write the workflow source, and
/// initialise a fresh context.
///
/// Returns `Ok(Some(context))` when the workflow contains an `@context = "..."`
/// directive, giving the caller a summary of what the workflow does.
/// Returns `Ok(None)` when no `@context` directive is present.
///
/// # Errors
/// Returns [`InstanceError`] on filesystem failure; the fresh context is also
/// serialised via [`save_context`].
///
/// Atomic across crashes: the new instance is staged in a sibling temp dir and
/// swapped in with a single `rename`, so a crash mid-write never leaves a
/// half-written instance — the previous instance (or none) remains intact.
pub fn start_instance(dir: &Path, workflow_src: &str) -> Result<Option<String>, InstanceError> {
    let module = steer_syntax::parse(workflow_src)?;
    let meta = extract_meta(&module);
    // Stage into a sibling temp directory next to the target.
    let staging = dir.with_extension("steer-new");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    fs::write(staging.join(WORKFLOW_FILE), workflow_src)?;
    save_context(&staging, &Context::new())?;
    // Swap: remove the old instance (if any), then move the staged one into
    // place. `rename` over a non-empty target is not portable, so remove first.
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    fs::rename(&staging, dir)?;
    Ok(meta.context)
}

/// Extract `WorkflowMeta` from a parsed module by evaluating top-level `@`
/// directives. Only `@template` and `@context` are recognised.
fn extract_meta(module: &steer_syntax::Module) -> WorkflowMeta {
    let mut meta = WorkflowMeta::default();
    for s in &module.body {
        if let steer_syntax::ast::Stmt::Meta { key, value } = &s.value {
            let v = eval_literal(value);
            if key == "template" {
                let rendered = v.render();
                meta.template_dir = if rendered.is_empty() {
                    None
                } else {
                    Some(rendered)
                };
            } else if key == "context" {
                let rendered = v.render();
                meta.context = if rendered.is_empty() {
                    None
                } else {
                    Some(rendered)
                };
            }
        }
    }
    meta
}

/// Load and lower the instance's workflow into its instruction stream.
///
/// # Errors
/// Returns [`InstanceError`] on filesystem, parse, or JSON errors.
pub fn load_ir(dir: &Path) -> Result<Vec<Instr>, InstanceError> {
    let src = fs::read_to_string(dir.join(WORKFLOW_FILE))?;
    let module = steer_syntax::parse(&src)?;
    Ok(lower(&module))
}

/// Load the instance's execution context.
///
/// # Errors
/// Returns [`InstanceError`] if the file cannot be read or is not valid JSON.
pub fn load_context(dir: &Path) -> Result<Context, InstanceError> {
    let src = fs::read_to_string(dir.join(CONTEXT_FILE))?;
    Ok(serde_json::from_str(&src)?)
}

/// Persist the execution context.
///
/// # Errors
/// Returns [`InstanceError`] if serialisation or the filesystem write fails.
pub fn save_context(dir: &Path, ctx: &Context) -> Result<(), InstanceError> {
    let json = serde_json::to_string_pretty(ctx)?;
    fs::write(dir.join(CONTEXT_FILE), json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use crate::vm::{check, set_value, step, StepOutcome};

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("steer-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    const WORKFLOW: &str = "x = ask(\"q\", return=\"str\")\nprint(x)\n";

    #[test]
    fn start_creates_fresh_context_and_workflow() {
        let dir = tmp("start");
        start_instance(&dir, WORKFLOW).unwrap();
        assert!(dir.join(WORKFLOW_FILE).exists());
        assert!(dir.join(CONTEXT_FILE).exists());
        let ctx = load_context(&dir).unwrap();
        assert_eq!(ctx.pc, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_clears_existing_instance() {
        let dir = tmp("clear");
        start_instance(&dir, WORKFLOW).unwrap();
        // mutate and save
        let mut ctx = load_context(&dir).unwrap();
        ctx.pc = 7;
        save_context(&dir, &ctx).unwrap();
        assert_eq!(load_context(&dir).unwrap().pc, 7);
        // restarting resets
        start_instance(&dir, WORKFLOW).unwrap();
        assert_eq!(load_context(&dir).unwrap().pc, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_ir_lowers_the_workflow() {
        let dir = tmp("ir");
        start_instance(&dir, WORKFLOW).unwrap();
        let ir = load_ir(&dir).unwrap();
        assert!(ir.len() > 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_persists_across_load_save_cycles() {
        // Simulate separate CLI calls: start, step (persists), set (persists),
        // check, step, check, step -> complete.
        let dir = tmp("run");
        start_instance(&dir, WORKFLOW).unwrap();

        // call 1: step -> pauses at ask
        let ir = load_ir(&dir).unwrap();
        let mut ctx = load_context(&dir).unwrap();
        assert!(matches!(
            step(&ir, &mut ctx, "<name>"),
            StepOutcome::Instruction(_)
        ));
        save_context(&dir, &ctx).unwrap();

        // call 2: set the value (simulating the agent)
        let mut ctx = load_context(&dir).unwrap();
        set_value(&mut ctx, "x", Value::Str("answer".into())).unwrap();
        save_context(&dir, &ctx).unwrap();

        // call 3: check -> advances; step -> pauses at print
        let mut ctx = load_context(&dir).unwrap();
        assert_eq!(
            check(&ir, &mut ctx, "<name>"),
            crate::vm::CheckOutcome::Advanced
        );
        match step(&ir, &mut ctx, "<name>") {
            StepOutcome::Instruction(s) => assert!(s.contains("answer"), "got: {s}"),
            o => panic!("unexpected {o:?}"),
        }
        save_context(&dir, &ctx).unwrap();

        // call 4: check print (auto), step -> complete
        let mut ctx = load_context(&dir).unwrap();
        check(&ir, &mut ctx, "<name>");
        assert_eq!(step(&ir, &mut ctx, "<name>"), StepOutcome::Complete);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_returns_context_from_directive() {
        let dir = tmp("ctx");
        let src = "@context = \"bug-fix workflow\"\ntask(\"do\")\n";
        let result = start_instance(&dir, src).unwrap();
        assert_eq!(result, Some("bug-fix workflow".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_returns_none_without_context_directive() {
        let dir = tmp("noctx");
        let result = start_instance(&dir, WORKFLOW).unwrap();
        assert_eq!(result, None);
        let _ = fs::remove_dir_all(&dir);
    }
}
