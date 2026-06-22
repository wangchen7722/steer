//! Lowering from the AST to a flat instruction stream.
//!
//! The interpreter steps a [`Module`] one action node at a
//! time, pausing for the agent between them. To make that resumable across CLI
//! invocations, the program is compiled to a flat [`Vec<Instr>`] whose program
//! counter is simply an index. Control flow becomes explicit jumps; `for` keeps
//! its iteration state in a hidden slot (`loop ... until` is a plain back-edge
//! with no slot); function calls record a return address in a call stack.
//!
//! `return` is one instruction: inside a function it pops a frame, at the top
//! level it halts the run. A top-level return value, if any, is ignored by design.

use std::collections::HashMap;

use steer_syntax::ast::{Call, CallArg, Expr, Stmt};
use steer_syntax::{Block, Module, Spanned};

/// A single instruction in the lowered program. Targets are indices into the
/// instruction vector.
#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    /// Update workflow metadata at runtime.
    SetMeta { key: String, expr: Expr },
    /// An action node such as `task`, `ask`, or `command`. `into` is the variable that
    /// receives its value when it is assigned or returned. Blocks until the
    /// agent reports via `check`/`set`.
    AgentOp { call: Call, into: Option<String> },
    /// Evaluate `expr` and bind it to `var`.
    Assign { var: String, expr: Expr },
    /// Jump to `target` if `cond` is falsy.
    JumpIfFalse { cond: Expr, target: u32 },
    /// Unconditional jump to `target`.
    Jump { target: u32 },
    /// Initialise a `for` loop: evaluate `list` into the hidden slot `iter`.
    ForInit { iter: String, list: Expr },
    /// `for` iteration: pop the next element of slot `iter` into `var`, or jump
    /// to `end` when the slot is exhausted.
    ForIter { iter: String, var: String, end: u32 },
    /// Call a user function: bind `params` to `args`, jump to `entry`; the
    /// return value binds to `into` when present.
    Call {
        entry: u32,
        params: Vec<String>,
        args: Vec<Expr>,
        into: Option<String>,
    },
    /// Return from a function (or halt the run at the top level).
    Return { value: Option<Expr> },
    /// End of program.
    Halt,
}

/// Lower a module into a flat instruction stream.
///
/// # Panics
/// This function does not panic on well-formed input. The call-site resolution
/// guard is `unreachable!`: every `Call` instruction is emitted only for a
/// callee present in `func_params`, which is built from the same definitions as
/// `func_entries`, so the entry lookup cannot miss.
pub fn lower(module: &Module) -> Vec<Instr> {
    // Collect top-level function definitions (name -> params, body).
    let funcs: Vec<(String, Vec<String>, Block)> = module
        .body
        .iter()
        .filter_map(|s| match &s.value {
            Stmt::Function { name, params, body } => {
                Some((name.clone(), params.clone(), body.clone()))
            }
            _ => None,
        })
        .collect();

    let mut l = Lowerer {
        code: Vec::new(),
        func_params: HashMap::new(),
        func_entries: HashMap::new(),
        call_sites: Vec::new(),
        loops: 0,
    };
    for (name, params, _body) in &funcs {
        l.func_params.insert(name.clone(), params.clone());
    }

    // Top-level statements (function definitions are emitted after Halt).
    for s in &module.body {
        if matches!(s.value, Stmt::Function { .. }) {
            continue;
        }
        l.lower_stmt(s);
    }
    l.emit(Instr::Halt);

    // Function bodies, each reachable only via a Call.
    for (name, _params, body) in &funcs {
        l.func_entries.insert(name.clone(), l.code.len() as u32);
        l.lower_block(body);
        if !matches!(l.code.last(), Some(Instr::Return { .. })) {
            l.emit(Instr::Return { value: None });
        }
    }

    // Resolve forward-referenced function calls. Every call site was emitted
    // only for a callee present in `func_params`, which is built from the same
    // definitions as `func_entries`, so the lookup cannot miss.
    for (idx, fname) in &l.call_sites {
        if let Instr::Call { entry, .. } = &mut l.code[*idx] {
            *entry = match l.func_entries.get(fname) {
                Some(&e) => e,
                None => {
                    unreachable!(
                        "call-site callee `{fname}` is in func_params, hence in func_entries"
                    )
                }
            };
        }
    }

    l.code
}

struct Lowerer {
    code: Vec<Instr>,
    func_params: HashMap<String, Vec<String>>,
    func_entries: HashMap<String, u32>,
    call_sites: Vec<(usize, String)>,
    loops: u32,
}

impl Lowerer {
    fn emit(&mut self, instr: Instr) -> usize {
        let idx = self.code.len();
        self.code.push(instr);
        idx
    }

    fn patch(&mut self, idx: usize, target: u32) {
        match &mut self.code[idx] {
            Instr::JumpIfFalse { target: t, .. }
            | Instr::Jump { target: t }
            | Instr::ForIter { end: t, .. } => *t = target,
            _ => {}
        }
    }

    fn fresh_slot(&mut self, prefix: &str) -> String {
        self.loops += 1;
        format!("__{prefix}_{}", self.loops)
    }

    fn lower_block(&mut self, block: &Block) {
        for s in block {
            self.lower_stmt(s);
        }
    }

    fn lower_stmt(&mut self, s: &Spanned<Stmt>) {
        match &s.value {
            Stmt::Meta { key, value } => {
                self.emit(Instr::SetMeta {
                    key: key.clone(),
                    expr: value.value.clone(),
                });
            }
            Stmt::Assign { target, value } => self.lower_value(value, Some(target.clone())),
            Stmt::Call(call) => {
                // A call statement runs for its effects: a user function lowers
                // to a `Call` instruction, anything else to an `AgentOp`.
                if self.func_params.contains_key(&call.callee) {
                    self.lower_call(call, None);
                } else {
                    self.emit(Instr::AgentOp {
                        call: call.clone(),
                        into: None,
                    });
                }
            }
            Stmt::Return { value } => match value {
                None => {
                    self.emit(Instr::Return { value: None });
                }
                Some(v) => {
                    // A returned call (user function or agent op) lowers to a call
                    // into a temp, then returns that temp — so an agent-op return
                    // pauses for the agent rather than being eval'd (and rejected)
                    // as a bare call expression.
                    if let Expr::Call(_) = &v.value {
                        let temp = "__ret".to_string();
                        self.lower_value(v, Some(temp.clone()));
                        self.emit(Instr::Return {
                            value: Some(Expr::Var(temp)),
                        });
                        return;
                    }
                    self.emit(Instr::Return {
                        value: Some(v.value.clone()),
                    });
                }
            },
            Stmt::If {
                branches,
                else_block,
            } => {
                // One `JumpIfFalse` per branch (skip to the next test when
                // false). A taken branch jumps past everything that follows to
                // the end — except the last branch when there is no `else`, which
                // simply falls through (no redundant jump).
                let mut jif_idxs: Vec<usize> = Vec::with_capacity(branches.len());
                let mut done_jumps: Vec<usize> = Vec::new();
                for (i, b) in branches.iter().enumerate() {
                    let jif = self.emit(Instr::JumpIfFalse {
                        cond: b.cond.value.clone(),
                        target: 0,
                    });
                    jif_idxs.push(jif);
                    self.lower_block(&b.body);
                    let is_last = i + 1 == branches.len();
                    if !is_last || else_block.is_some() {
                        done_jumps.push(self.emit(Instr::Jump { target: 0 }));
                    }
                }
                // Each test skips to the next branch's test, or — for the last
                // branch — to the `else` block (the end, when there is none).
                let fallthrough = self.code.len() as u32;
                for window in jif_idxs.windows(2) {
                    self.patch(window[0], window[1] as u32);
                }
                if let Some(&last) = jif_idxs.last() {
                    self.patch(last, fallthrough);
                }
                if let Some(eb) = else_block {
                    self.lower_block(eb);
                }
                let end = self.code.len() as u32;
                for j in done_jumps {
                    self.patch(j, end);
                }
            }
            Stmt::LoopUntil { body, cond } => {
                let start = self.code.len() as u32;
                self.lower_block(body);
                self.emit(Instr::JumpIfFalse {
                    cond: cond.value.clone(),
                    target: start,
                });
            }
            Stmt::For {
                var,
                iterable,
                body,
            } => {
                let iter = self.fresh_slot("for");
                self.emit(Instr::ForInit {
                    iter: iter.clone(),
                    list: iterable.value.clone(),
                });
                let it = self.emit(Instr::ForIter {
                    iter,
                    var: var.clone(),
                    end: 0,
                });
                self.lower_block(body);
                self.emit(Instr::Jump { target: it as u32 });
                self.patch(it, self.code.len() as u32);
            }
            Stmt::Function { .. } => {} // emitted after Halt
        }
    }

    /// Lower a value-position expression: an assignment RHS, a bare statement,
    /// or a return value. A call is either a user-function `Call` instruction
    /// or an `AgentOp`; anything else is an `Assign`, which is only meaningful
    /// with a target.
    fn lower_value(&mut self, e: &Spanned<Expr>, into: Option<String>) {
        match &e.value {
            Expr::Call(c) if self.func_params.contains_key(&c.callee) => {
                self.lower_call(c, into);
            }
            Expr::Call(c) => {
                self.emit(Instr::AgentOp {
                    call: c.clone(),
                    into,
                });
            }
            other => {
                if let Some(var) = into {
                    self.emit(Instr::Assign {
                        var,
                        expr: other.clone(),
                    });
                }
                // a bare non-call expression statement is a no-op
            }
        }
    }

    fn lower_call(&mut self, call: &Call, into: Option<String>) {
        let params = self
            .func_params
            .get(&call.callee)
            .cloned()
            .unwrap_or_default();
        let args: Vec<Expr> = call
            .args
            .iter()
            .filter_map(|a| match &a.value {
                CallArg::Positional(e) => Some(e.value.clone()),
                CallArg::Named { .. } => None, // named args to user functions are not supported in v1
            })
            .collect();
        let idx = self.emit(Instr::Call {
            entry: 0,
            params,
            args,
            into,
        });
        self.call_sites.push((idx, call.callee.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ir(src: &str) -> Vec<Instr> {
        let m = steer_syntax::parse(src).expect("parses");
        lower(&m)
    }

    fn kinds(ir: &[Instr]) -> Vec<&'static str> {
        ir.iter()
            .map(|i| match i {
                Instr::AgentOp { .. } => "agent",
                Instr::SetMeta { .. } => "meta",
                Instr::Assign { .. } => "assign",
                Instr::JumpIfFalse { .. } => "jif",
                Instr::Jump { .. } => "j",
                Instr::ForInit { .. } => "forinit",
                Instr::ForIter { .. } => "foriter",
                Instr::Call { .. } => "call",
                Instr::Return { .. } => "ret",
                Instr::Halt => "halt",
            })
            .collect()
    }

    #[test]
    fn bare_task_then_halt() {
        assert_eq!(kinds(&ir("task(\"do\")\n")), vec!["agent", "halt"]);
    }

    #[test]
    fn meta_lowers_to_set_meta() {
        assert_eq!(
            kinds(&ir("@template = \"bugfix\"\ntask(\"do\")\n")),
            vec!["meta", "agent", "halt"]
        );
    }

    #[test]
    fn assign_literal() {
        assert_eq!(kinds(&ir("x = 5\n")), vec!["assign", "halt"]);
    }

    #[test]
    fn value_task_assigned_is_agent_op_with_target() {
        let code = ir("x = command(\"ls\", return=\"list\")\n");
        assert!(matches!(code[0], Instr::AgentOp { into: Some(_), .. }));
    }

    #[test]
    fn if_else_lowers_to_jif_jump() {
        let code = ir("if x\n  print(\"a\")\nelse\n  print(\"b\")\nend\n");
        assert_eq!(kinds(&code), vec!["jif", "agent", "j", "agent", "halt"]);
        // the jif must skip the then-branch into the else-branch
        if let Instr::JumpIfFalse { target, .. } = &code[0] {
            assert_eq!(*target, 3); // lands on the else print
        }
        // the unconditional jump after then must skip the else-branch to halt
        if let Instr::Jump { target } = &code[2] {
            assert_eq!(*target, 4);
        }
    }

    #[test]
    fn if_without_else() {
        let code = ir("if x\n  print(\"a\")\nend\n");
        assert_eq!(kinds(&code), vec!["jif", "agent", "halt"]);
        if let Instr::JumpIfFalse { target, .. } = &code[0] {
            assert_eq!(*target, 2); // skip straight to halt
        }
    }

    #[test]
    fn elseif_lowers_to_branch_chain() {
        let code =
            ir("if a\n  print(\"1\")\nelseif b\n  print(\"2\")\nelse\n  print(\"3\")\nend\n");
        // jif(a), branch A, jump; jif(b), branch B, jump; else branch C; halt
        assert_eq!(
            kinds(&code),
            vec!["jif", "agent", "j", "jif", "agent", "j", "agent", "halt"]
        );
        // jif(a) skips to jif(b); jif(b) skips to the else branch.
        match &code[0] {
            Instr::JumpIfFalse { target, .. } => assert_eq!(*target, 3),
            _ => panic!("expected jif at 0"),
        }
        match &code[3] {
            Instr::JumpIfFalse { target, .. } => assert_eq!(*target, 6),
            _ => panic!("expected jif at 3"),
        }
        // both taken-branch jumps land past the else (on halt).
        assert!(matches!(&code[2], Instr::Jump { target: 7 }));
        assert!(matches!(&code[5], Instr::Jump { target: 7 }));
    }

    #[test]
    fn loop_until_back_edge() {
        let code = ir("loop\n  x = 1\nuntil x > 5\n");
        // assign, then jif back to the assign
        assert_eq!(kinds(&code), vec!["assign", "jif", "halt"]);
        if let Instr::JumpIfFalse { target, .. } = &code[1] {
            assert_eq!(*target, 0);
        }
    }

    #[test]
    fn for_in_loop() {
        let code = ir("for f in items\n  print(f)\nend\n");
        assert_eq!(
            kinds(&code),
            vec!["forinit", "foriter", "agent", "j", "halt"]
        );
        if let Instr::ForIter { end, .. } = &code[1] {
            assert_eq!(*end, 4);
        }
    }

    #[test]
    fn function_call_resolves_entry_after_halt() {
        let code = ir("func f()\n  print(\"in f\")\nend\nf()\n");
        // top-level: call, halt, then func body: agent, ret
        assert_eq!(kinds(&code), vec!["call", "halt", "agent", "ret"]);
        // the call's entry must point at the function body (index 2)
        if let Instr::Call { entry, .. } = &code[0] {
            assert_eq!(*entry, 2);
        }
    }

    #[test]
    fn function_with_return_value_into_target() {
        let code = ir("func g()\n  return \"x\"\nend\nr = g()\n");
        // body already ends in `return`, so no trailing Return is appended
        assert_eq!(kinds(&code), vec!["call", "halt", "ret"]);
        if let Instr::Call { into, .. } = &code[0] {
            assert_eq!(into.as_deref(), Some("r"));
        }
    }

    #[test]
    fn example_bugfix_loop_lowers_without_panic() {
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
        let code = ir(src);
        assert!(code.len() > 10);
        // function bodies are emitted after Halt, so Halt need not be last
        assert!(code.iter().any(|i| matches!(i, Instr::Halt)));
        assert!(code.iter().any(|i| matches!(i, Instr::Return { .. })));
    }

    #[test]
    fn empty_module_is_just_halt() {
        assert_eq!(kinds(&ir("// nothing\n")), vec!["halt"]);
    }
}
