//! The execution context: the resumable state of a run.
//!
//! A [`Context`] is everything the interpreter needs to resume a run between
//! CLI invocations. It is serialised to `context.json` (in a later milestone);
//! for now it is an in-memory value the VM operates on.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::value::Value;

/// Run status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// The run is in progress.
    Running,
    /// The run reached `Halt` or a top-level `return`.
    Complete,
    /// The agent reported a fatal error via `steer error`.
    Halted(String),
}

/// A call-stack frame for a user-function call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Where to resume in the caller.
    pub return_pc: u32,
    /// The caller variable that receives the return value, if any.
    pub into: Option<String>,
    /// The caller's variable scope, saved for restoration on return.
    pub saved_vars: HashMap<String, Value>,
}

/// Per-agent-op state, keyed by the op's program-counter index.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StepState {
    /// The agent's verification result, when the op has a `check`.
    pub checked: Option<CheckedReport>,
    /// The most recent failed verification reason for retry context.
    pub failure_reason: Option<String>,
    /// How many times this op's check has failed (used for retry context and
    /// auto-halt after excessive retries).
    #[serde(default)]
    pub retry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CheckedReport {
    /// Backward-compatible pass-only form from `steer set checked true`.
    Bool(bool),
    /// Structured verification report.
    Object {
        passed: bool,
        reason: Option<String>,
    },
}

impl CheckedReport {
    pub fn passed(&self) -> bool {
        match self {
            CheckedReport::Bool(passed) => *passed,
            CheckedReport::Object { passed, .. } => *passed,
        }
    }

    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            CheckedReport::Bool(_) => None,
            CheckedReport::Object { reason, .. } => reason.as_deref(),
        }
    }
}

/// Runtime workflow metadata controlled by `@` directives.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkflowMeta {
    /// Active template directory under `.steer/templates/`. `None` means `default`.
    pub template_dir: Option<String>,
    /// Workflow-level context description set by `@context = "..."`.
    #[serde(default)]
    pub context: Option<String>,
}

/// The full execution context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    /// Program counter: index into the lowered instruction stream.
    pub pc: u32,
    pub status: Status,
    /// The current scope's variables.
    pub vars: HashMap<String, Value>,
    /// The call stack: one frame per active user-function call.
    pub frames: Vec<Frame>,
    /// Per-agent-op state, keyed by program-counter index.
    pub steps: HashMap<u32, StepState>,
    /// Runtime metadata set by `@` directives.
    #[serde(default)]
    pub meta: WorkflowMeta,
}

impl Context {
    /// A fresh context at the start of a run.
    #[must_use]
    pub fn new() -> Self {
        Context {
            pc: 0,
            status: Status::Running,
            vars: HashMap::new(),
            frames: Vec::new(),
            steps: HashMap::new(),
            meta: WorkflowMeta::default(),
        }
    }

    /// Whether the run is still in progress.
    pub fn is_running(&self) -> bool {
        self.status == Status::Running
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_context_starts_running_at_zero() {
        let c = Context::new();
        assert_eq!(c.pc, 0);
        assert!(c.is_running());
        assert!(c.vars.is_empty());
    }

    #[test]
    fn context_round_trips_through_json() {
        let mut c = Context::new();
        c.pc = 3;
        c.vars.insert("x".into(), Value::Int(5));
        c.meta.template_dir = Some("bugfix".into());
        c.meta.context = Some("bug-fix workflow".into());
        c.steps.insert(
            2,
            StepState {
                checked: Some(CheckedReport::Object {
                    passed: true,
                    reason: None,
                }),
                failure_reason: Some("previous failure".into()),
                retry_count: 3,
            },
        );
        let json = serde_json::to_string(&c).unwrap();
        let back: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }
}
