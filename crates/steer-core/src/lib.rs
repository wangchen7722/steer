//! Core runtime of steer: validation, template rendering, simulation, IR
//! lowering, the interpreter, and instance persistence.
//!
//! steer is a tiny control unit (PC): it walks a workflow program and renders
//! instructions for an external coding agent. It never touches the outside
//! world itself — all execution (shell, files, user interaction, verification)
//! is delegated to the agent and flows back via `steer set` / `steer error`.
//!
//! This crate builds on [`steer_syntax`].

#![forbid(unsafe_code)]
// Lint policy: the hard gate is `cargo clippy --all-targets -- -D warnings`
// (clippy's default set). The pedantic lints allowed below are deliberate
// project choices, documented here so they are not mistaken for oversights.
#![allow(
    clippy::cast_possible_truncation, // the program counter and IR indices are u32 by design
    clippy::cast_precision_loss,      // i64 -> f64 in numeric evaluation is intended
    clippy::too_many_lines,           // `vm::step` is one coherent interpreter loop
    clippy::module_name_repetitions,  // re-exports keep call-site names explicit
    clippy::wildcard_imports,         // `use crate::ast::*` is intentional AST ergonomics
    clippy::implicit_hasher,          // internal maps use the default hasher
    clippy::many_single_char_names,   // short loop counters and match binders (i, n, s)
    clippy::must_use_candidate,       // `#[must_use]` is added only where dropping is a bug
    clippy::single_char_pattern,      // single-char string patterns read fine in rendering
    clippy::enum_glob_use,            // tight operator-dispatch tables use `use Op::*`
    clippy::single_match_else,        // a two-arm `match` is often clearer than `if/else`
    clippy::if_not_else,              // condition polarity follows the surrounding logic
    clippy::map_unwrap_or             // `.map(f).unwrap_or(a)` is preferred here for order
)]

pub mod context;
pub mod ir;
pub mod simulate;
pub mod storage;
pub mod template;
pub mod validate;
pub mod value;
pub mod vm;

pub use context::{Context, Frame, Status, StepState};
pub use ir::{lower, Instr};
pub use simulate::{simulate, SimStep};
pub use storage::{load_context, load_ir, save_context, start_instance, InstanceError};
pub use template::{
    render_call, render_check_report, render_retry_context, render_start_output,
    render_status_output, Template, TemplateError,
};
pub use validate::{validate, Diagnostic, Severity};
pub use value::{eval, eval_literal, parse_value, EvalError, Value};
pub use vm::{check, report_error, set_value, step, CheckOutcome, StepOutcome};
