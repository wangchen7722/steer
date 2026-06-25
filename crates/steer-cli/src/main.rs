//! `steer` command-line entry point.
//!
//! The command surface mirrors the design: `steer <resource> <action> <args>`,
//! where a resource is either a [`Workflow`](Resource::Workflow) (a definition)
//! or an [`Instance`](Resource::Instance) (a run).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// steer — a tiny control unit that drives an external coding agent through
/// declarative, verifiable workflows.
#[derive(Parser)]
#[command(name = "steer", version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    resource: Resource,
}

/// Top-level resources.
#[derive(Subcommand)]
enum Resource {
    /// Author and debug workflows without a running instance.
    Workflow(WorkflowArgs),
    /// Run lifecycle and execution loop for a named instance.
    Instance(InstanceArgs),
}

#[derive(Parser)]
struct WorkflowArgs {
    #[command(subcommand)]
    action: WorkflowAction,
}

#[derive(Subcommand)]
enum WorkflowAction {
    /// Statically validate a workflow file: syntax and semantic checks.
    Validate {
        /// Path to the `.steer` workflow file.
        workflow: PathBuf,
    },
    /// Dry-run: render every task instruction with a mock agent and print them.
    Simulate {
        /// Path to the `.steer` workflow file.
        workflow: PathBuf,
    },
    /// List workflows in a directory (default `.steer/workflows/`), printing each
    /// workflow's name with its `@description`.
    List {
        /// Directory to scan for `*.steer` files. Defaults to `.steer/workflows/`.
        dir: Option<PathBuf>,
    },
}

#[derive(Parser)]
struct InstanceArgs {
    #[command(subcommand)]
    action: InstanceAction,
}

#[derive(Subcommand)]
enum InstanceAction {
    /// Create or reset an instance and initialise its program counter.
    Start {
        workflow: PathBuf,
        /// Instance name, the folder under `.steer/instances/`.
        name: String,
    },
    /// Show the status of an instance.
    Status { name: String },
    /// Return the instruction at the current program counter, without changing state.
    Step { name: String },
    /// Advance the program counter, dispatching by node type.
    Check { name: String },
    /// Write a typed value/flag into the instance context.
    Set {
        name: String,
        var: String,
        value: String,
    },
    /// Report a fatal failure; the instance halts immediately.
    Error { name: String, reason: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.resource {
        Resource::Workflow(w) => match w.action {
            WorkflowAction::Validate { workflow } => run_validate(&workflow),
            WorkflowAction::Simulate { workflow } => run_simulate(&workflow),
            WorkflowAction::List { dir } => run_list(dir.as_deref()),
        },
        Resource::Instance(i) => match i.action {
            InstanceAction::Start { workflow, name } => run_instance_start(&workflow, &name),
            InstanceAction::Status { name } => run_instance_status(&name),
            InstanceAction::Step { name } => run_instance_step(&name),
            InstanceAction::Check { name } => run_instance_check(&name),
            InstanceAction::Set { name, var, value } => run_instance_set(&name, &var, &value),
            InstanceAction::Error { name, reason } => run_instance_error(&name, &reason),
        },
    }
}

/// Path of an instance directory under the current working directory. The name
/// is sanitized so a value like `../x` cannot escape `.steer/instances/`.
fn instance_dir(name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(format!("invalid instance name `{name}`"));
    }
    Ok(PathBuf::from(".steer").join("instances").join(name))
}

/// Resolve a workflow path argument to an existing file.
///
/// Order: the path as given (CWD, absolute, or explicit relative) first; if
/// that is not a regular file, fall back to a flat lookup under
/// `.steer/workflows/` by file name, auto-appending `.steer` when the given
/// name has no extension. Returns the original argument when nothing matches,
/// so the caller's "cannot read" error stays informative and backward
/// compatible.
fn resolve_workflow(arg: &Path) -> PathBuf {
    if arg.is_file() {
        return arg.to_path_buf();
    }
    let Some(file_name) = arg.file_name() else {
        return arg.to_path_buf();
    };
    let workflows = Path::new(".steer/workflows");
    let exact = workflows.join(file_name);
    if exact.is_file() {
        return exact;
    }
    // Tolerate a missing `.steer` extension: `bugfix-loop` -> `bugfix-loop.steer`.
    if Path::new(file_name).extension().is_none() {
        let with_ext = workflows.join(file_name).with_extension("steer");
        if with_ext.is_file() {
            return with_ext;
        }
    }
    arg.to_path_buf()
}

/// Run `steer instance start <workflow> <name>`: validate, then create the
/// instance with a fresh context.
fn run_instance_start(workflow: &Path, name: &str) -> ExitCode {
    let workflow = resolve_workflow(workflow);
    let src = match std::fs::read_to_string(&workflow) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", workflow.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = steer_syntax::parse(&src) {
        let (line, col) = steer_syntax::line_col(&src, e.span.start);
        eprintln!("error: {e} (at line {line}, col {col})");
        return ExitCode::FAILURE;
    }
    let dir = match instance_dir(name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    match steer_core::start_instance(&dir, &src) {
        Ok(context_desc) => {
            println!(
                "{}",
                steer_core::render_start_output(name, context_desc.as_deref())
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Load an instance's IR and context, apply `f`, persist the context, and print
/// the returned message.
fn with_instance(
    name: &str,
    f: impl FnOnce(&[steer_core::Instr], &mut steer_core::Context, &str) -> String,
) -> ExitCode {
    with_instance_result(name, |ir, ctx, n| Ok(f(ir, ctx, n)))
}

fn with_instance_result(
    name: &str,
    f: impl FnOnce(&[steer_core::Instr], &mut steer_core::Context, &str) -> Result<String, String>,
) -> ExitCode {
    let dir = match instance_dir(name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let ir = match steer_core::load_ir(&dir) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut ctx = match steer_core::load_context(&dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let msg = match f(&ir, &mut ctx, name) {
        Ok(msg) => msg,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = steer_core::save_context(&dir, &ctx) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    println!("{msg}");
    ExitCode::SUCCESS
}

fn run_instance_step(name: &str) -> ExitCode {
    with_instance(name, |ir, ctx, n| match steer_core::step(ir, ctx, n) {
        steer_core::StepOutcome::Instruction(s) => s,
        steer_core::StepOutcome::Complete => "(complete)".to_string(),
        steer_core::StepOutcome::NotRunning => "(not running)".to_string(),
        steer_core::StepOutcome::Error(e) => format!("error: {e}"),
    })
}

fn run_instance_check(name: &str) -> ExitCode {
    with_instance(name, |ir, ctx, n| match steer_core::check(ir, ctx, n) {
        steer_core::CheckOutcome::Advanced => "advanced".to_string(),
        steer_core::CheckOutcome::Instruction(s) => s,
        steer_core::CheckOutcome::Pending => "pending".to_string(),
        steer_core::CheckOutcome::Failed => "failed".to_string(),
        steer_core::CheckOutcome::Done => "(done)".to_string(),
        steer_core::CheckOutcome::NotRunning => "(not running)".to_string(),
        steer_core::CheckOutcome::Error(e) => format!("error: {e}"),
    })
}

fn run_instance_set(name: &str, var: &str, value: &str) -> ExitCode {
    let parsed = steer_core::parse_value(value);
    with_instance_result(name, |ir, ctx, _| {
        // Enforce the current op's declared `return` type at set time: if `var`
        // is the assignment target of the op at `ctx.pc`, the value must match
        // the callee's declared type. A mismatch is rejected before storing, so
        // a wrong-typed value can never reach a downstream condition.
        steer_core::validate_set_value(ir, ctx, var, &parsed)?;
        steer_core::set_value(ctx, var, parsed)?;
        Ok("ok".to_string())
    })
}

fn run_instance_error(name: &str, reason: &str) -> ExitCode {
    with_instance(name, |_ir, ctx, _| {
        steer_core::report_error(ctx, reason);
        "halted".to_string()
    })
}

fn run_instance_status(name: &str) -> ExitCode {
    let dir = match instance_dir(name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let ctx = match steer_core::load_context(&dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let status = match ctx.status {
        steer_core::Status::Running => "running".to_string(),
        steer_core::Status::Complete => "complete".to_string(),
        steer_core::Status::Halted(r) => format!("halted: {r}"),
    };
    println!(
        "{}",
        steer_core::render_status_output(name, &status, ctx.meta.context.as_deref())
    );
    ExitCode::SUCCESS
}

/// Read and parse a workflow file, printing a located error on failure.
fn load_workflow(path: &Path) -> Result<(String, steer_syntax::Module), ExitCode> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        ExitCode::FAILURE
    })?;
    let module = steer_syntax::parse(&src).map_err(|e| {
        let (line, col) = steer_syntax::line_col(&src, e.span.start);
        eprintln!("error: {e} (at line {line}, col {col})");
        ExitCode::FAILURE
    })?;
    Ok((src, module))
}

/// Run `steer workflow validate <path>`: parse + semantic-check the workflow.
fn run_validate(path: &Path) -> ExitCode {
    let path = resolve_workflow(path);
    let (src, module) = match load_workflow(&path) {
        Ok(x) => x,
        Err(code) => return code,
    };
    let diags = steer_core::validate(&module);
    if diags.is_empty() {
        println!("{}: OK", path.display());
        ExitCode::SUCCESS
    } else {
        for d in &diags {
            let (line, col) = steer_syntax::line_col(&src, d.span.start);
            eprintln!("error: {} (at line {line}, col {col})", d.message);
        }
        eprintln!("{}: {} error(s)", path.display(), diags.len());
        ExitCode::FAILURE
    }
}

/// Run `steer workflow simulate <path>`: render every instruction in order.
fn run_simulate(path: &Path) -> ExitCode {
    let path = resolve_workflow(path);
    let (_src, module) = match load_workflow(&path) {
        Ok(x) => x,
        Err(code) => return code,
    };
    let steps = steer_core::simulate(&module);
    if steps.is_empty() {
        println!("(no action nodes)");
    } else {
        for (i, step) in steps.iter().enumerate() {
            println!("[{}] {}", i + 1, step.callee);
            println!("{}", step.instruction.trim_end());
            println!();
        }
    }
    ExitCode::SUCCESS
}

/// Run `steer workflow list [dir]`: enumerate `*.steer` workflows and print
/// each name with its `@description`. Defaults to `.steer/workflows/`.
///
/// Never fails on a bad entry: a missing description prints `(no description)`,
/// an unparseable file prints `(unparseable)`, an unreadable file prints
/// `(unreadable)`, and a missing/empty directory prints
/// `(no workflows in <dir>)` — all exit successfully.
fn run_list(dir: Option<&Path>) -> ExitCode {
    let dir = dir.unwrap_or_else(|| Path::new(".steer/workflows"));
    let mut rows: Vec<(String, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("steer") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            rows.push((stem.to_string(), read_description(&path)));
        }
    }
    if rows.is_empty() {
        println!("(no workflows in {})", dir.display());
        return ExitCode::SUCCESS;
    }
    // File stems are unique within a directory, so tuple ordering sorts by name.
    rows.sort();
    let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, desc) in &rows {
        println!("{name:<width$}  {desc}");
    }
    ExitCode::SUCCESS
}

/// Read and parse `path`, returning its `@description` text or a placeholder.
fn read_description(path: &Path) -> String {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return "(unreadable)".to_string(),
    };
    match steer_syntax::parse(&src) {
        Ok(module) => steer_core::workflow_description(&module)
            .unwrap_or_else(|| "(no description)".to_string()),
        Err(_) => "(unparseable)".to_string(),
    }
}
