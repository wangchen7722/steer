# IR Lowering

## Purpose

`lower` compiles a parsed `.steer` `Module` into a flat, index-resumable
`Vec<Instr>` so that the interpreter (see
[interpreter-execution](openspec/specs/interpreter-execution/spec.md)) can step it
purely by program counter. Control flow becomes explicit jumps, `for`-iteration
state lives in hidden slots, function bodies are emitted after a top-level
`Halt`, and every call target is resolved in a second pass so all jump and call
indices are valid.

## Requirements

### Requirement: Top-level statements lower in source order and terminate with Halt

`lower` SHALL emit top-level statements as instructions in source order, and the
top-level instruction stream SHALL end with `Instr::Halt`. `Stmt::Function` SHALL
NOT emit any instruction at top level; function bodies are emitted after `Halt`.
An empty module SHALL lower to exactly `[Halt]`. Evidenced by `lower` and the
tests `bare_task_then_halt`, `empty_module_is_just_halt` in
`crates/steer-core/src/ir.rs`.

#### Scenario: a sequence of top-level statements is followed by Halt

- **WHEN** a module contains several top-level statements in order
- **THEN** the lowered instruction stream contains the corresponding instructions
  in source order followed by a single trailing `Halt`.

#### Scenario: an empty module lowers to just Halt

- **WHEN** a module contains no statements
- **THEN** `lower` returns exactly `[Instr::Halt]`.

### Requirement: Each statement kind maps to a fixed instruction

`lower` SHALL map each statement kind to a deterministic instruction:
`Stmt::Meta` to `Instr::SetMeta { key, expr }`; an assignment whose right-hand
side is a user-function call to `Instr::Call`; any other assignment to
`Instr::Assign { var, expr }`; a bare user-function call statement to
`Instr::Call` with `into = None`; and a bare agent-node call statement to
`Instr::AgentOp` with `into = None`. Evidenced by `lower` and the tests
`meta_lowers_to_set_meta` in `crates/steer-core/src/ir.rs`.

#### Scenario: a meta statement lowers to SetMeta

- **WHEN** a module contains a `Stmt::Meta` with a key and expression
- **THEN** `lower` emits an `Instr::SetMeta { key, expr }` instruction.

#### Scenario: a bare agent call lowers to AgentOp

- **WHEN** a module contains a bare agent-node call statement with no receiver
- **THEN** `lower` emits an `Instr::AgentOp` with `into = None`.

### Requirement: Control flow lowers to conditional and unconditional jumps

`lower` SHALL lower an `if`/`elseif`/`else` chain by emitting, per branch, a
`JumpIfFalse` that skips to the next branch test followed by the branch body and
an unconditional `Jump` to a common end, except that the last branch with no
`else` SHALL fall through with no redundant `Jump`. A `loop ... until cond` SHALL
lower to its body followed by `JumpIfFalse { cond, target: start }` as a
post-test, so the body executes at least once. Evidenced by `lower` and the tests
`if_else_lowers_to_jif_jump`, `loop_until_back_edge` in
`crates/steer-core/src/ir.rs`.

#### Scenario: an if/else chain branches on JumpIfFalse and rejoins at end

- **WHEN** a module contains an `if`/`elseif`/`else` chain
- **THEN** each tested branch is guarded by a `JumpIfFalse`, and every branch
  body except the last unconditional fall-through ends with a `Jump` to the same
  end index.

#### Scenario: a loop until is a post-test back edge

- **WHEN** a module contains a `loop ... until cond`
- **THEN** the lowered body is followed by a `JumpIfFalse` whose target is the
  first instruction of the body, so the body runs at least once before the
  condition is tested.

### Requirement: For loops keep iteration state in hidden slots

`lower` SHALL lower `for var in iterable` into `ForInit { iter, list }` storing
the remaining list into a fresh hidden slot named `__for_<n>`, followed by
`ForIter { iter, var, end }`, the loop body, and an unconditional `Jump` back to
the `ForIter` index; the `ForIter.end` target SHALL be patched to the first
instruction after the loop. Evidenced by `lower`/`fresh_slot`/`patch` and the
test `for_in_loop` in `crates/steer-core/src/ir.rs`.

#### Scenario: a for loop reserves a hidden iteration slot

- **WHEN** a module contains `for var in iterable` over a list body
- **THEN** the lowered form begins with `ForInit` writing to a `__for_<n>` slot,
  followed by `ForIter`, the body, a `Jump` to the `ForIter`, and `ForIter.end`
  patched to the instruction after the loop.

### Requirement: Function bodies emit after Halt and return-encode agent calls

`lower` SHALL emit each function body after the top-level `Halt`, and each body
SHALL end with `Instr::Return`; a trailing `Return { value: None }` SHALL be
appended when the last statement is not a `Return`. A `return <expr>` where
`<expr>` is an `Expr::Call` SHALL first lower the call into a temp `__ret` slot
and then emit `return Expr::Var("__ret")`, so an agent-op return pauses for the
agent rather than being evaluated bare. Evidenced by `lower` and the tests
`function_call_resolves_entry_after_halt` in `crates/steer-core/src/ir.rs`.

#### Scenario: a function body is emitted after the top-level Halt

- **WHEN** a module declares a function and calls it at top level
- **THEN** the function body instructions appear after the top-level `Halt` and
  end with an `Instr::Return`.

#### Scenario: a trailing Return is appended when missing

- **WHEN** a function body's last statement is not a `return`
- **THEN** `lower` appends a `Return { value: None }` after the body.

### Requirement: All call entries and jump targets are resolved before return

`lower` SHALL resolve every `Instr::Call.entry` to the index of the callee's body
in a second pass, and every `Jump`/`JumpIfFalse`/`ForIter` target SHALL be a
valid instruction index. An unresolved callee SHALL be treated as an error path,
never an invalid index. Evidenced by the pass-2 resolution loop in `lower` and
the test `function_call_resolves_entry_after_halt` in
`crates/steer-core/src/ir.rs`.

#### Scenario: call entries point at function bodies resolved in pass 2

- **WHEN** a module calls a user function defined later in source order
- **THEN** `lower` resolves the call's `entry` to that function's body index
  during the second pass, and the index is within the instruction vector.
