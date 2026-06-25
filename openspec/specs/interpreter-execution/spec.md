# Interpreter Execution

## Purpose

`step` is the instruction-stepping interpreter that advances the program counter
through a lowered `Vec<Instr>` (see
[ir-lowering](openspec/specs/ir-lowering/spec.md)), pausing at the next agent
operation so an external agent can act, and halting on evaluation errors. The
interpreter holds no resumable state itself — all of PC, status, variables,
frames, and meta live in `Context` — so any run can be suspended and resumed.

## Requirements

### Requirement: step is a no-op when not running and completes past the end

`step` on a `Context` whose `Status` is not `Running` SHALL immediately return
`StepOutcome::NotRunning` without advancing the PC. When the PC points past the
end of the instruction vector, `step` SHALL set `Status::Complete` and return
`StepOutcome::Complete`. Evidenced by `step` and the `StepOutcome::NotRunning`
boundary check in `crates/steer-core/src/vm.rs`.

#### Scenario: stepping a halted or complete context does nothing

- **WHEN** `step` is invoked on a context whose status is `Complete` or `Halted`
- **THEN** it returns `NotRunning` and leaves the PC unchanged.

#### Scenario: running off the end completes the run

- **WHEN** the PC is past the last instruction while status is still `Running`
- **THEN** `step` sets `Status::Complete` and returns `Complete`.

### Requirement: step pauses at agent operations without advancing the PC

`step` SHALL, on an `Instr::AgentOp { call, into }`, render the instruction text
and return `StepOutcome::Instruction(text)` WITHOUT advancing the PC, so the
interpreter pauses at the agent op until `check` (see
[runtime-check-gate](openspec/specs/runtime-check-gate/spec.md)) advances it. If
a `StepState` with a `failure_reason` exists at the current PC, `step` SHALL
append retry context to the rendered instruction. Evidenced by the `AgentOp` arm
of `step` and the test `step_stops_at_first_agent_op` in
`crates/steer-core/src/vm.rs`.

#### Scenario: the first agent op stops the interpreter

- **WHEN** a run reaches an `AgentOp` while `Running`
- **THEN** `step` returns the rendered instruction text and does not advance the
  PC, leaving the op as the current instruction.

#### Scenario: a failing op re-renders with retry context

- **WHEN** `step` pauses on an op whose `StepState` carries a `failure_reason`
- **THEN** the returned instruction appends a retry block naming the reason and
  the current retry count.

### Requirement: Each control instruction advances the PC by its own rule

`step` SHALL apply per-instruction PC semantics: `Halt` sets `Status::Complete`
and returns `Complete`; `SetMeta` evaluates its expression, applies the meta, and
advances PC by one; `Assign` evaluates its expression, stores into the named
variable, and advances PC by one; `JumpIfFalse { cond, target }` sets the PC to
`target` when the condition is falsy and otherwise advances by one; `Jump {
target }` sets the PC to `target`. Evidenced by the corresponding arms of `step`
in `crates/steer-core/src/vm.rs`.

#### Scenario: JumpIfFalse branches on a false condition

- **WHEN** `step` executes `JumpIfFalse` whose condition evaluates falsy
- **THEN** the PC is set to the instruction's `target`.

#### Scenario: SetMeta applies the meta and advances

- **WHEN** `step` executes `SetMeta` whose expression evaluates cleanly
- **THEN** the meta is applied (e.g. `template` to `template_dir`, empty to
  `None`) and the PC advances by one.

### Requirement: ForIter iterates only Value lists

`step` SHALL, on `ForIter { iter, var, end }`, inspect `vars[iter]`: if it is a
non-empty `Value::List`, move the first element into `var`, store the remaining
list back into `iter`, and advance the PC by one; otherwise it SHALL set the PC
to `end`, ending the loop. The `for` loop SHALL iterate only `Value::List`; any
other iterable type SHALL end the loop immediately rather than raise an error.
Evidenced by the `ForIter` arm of `step` and the test `for_loop_iterations` in
`crates/steer-core/src/vm.rs`.

#### Scenario: a non-list iterable ends the loop

- **WHEN** `vars[iter]` is a `Value` other than a non-empty `List` at a `ForIter`
- **THEN** the PC is set to `end`, terminating the loop without error.

#### Scenario: a list yields its head and keeps the tail

- **WHEN** `vars[iter]` is a non-empty `Value::List` at a `ForIter`
- **THEN** the head is bound to `var`, the tail is stored back into `iter`, and
  the PC advances by one.

### Requirement: Calls push a frame and returns restore it

`step` SHALL, on `Instr::Call { entry, params, args, into }`, evaluate each
positional argument in the caller's variables, build a fresh scope, push a
`Frame { return_pc, into, saved_vars }` capturing the caller's variables, replace
the context variables with the new scope, and set the PC to `entry`. On
`Instr::Return { value }`, `step` SHALL evaluate the value (defaulting to
`Value::Null`), restore the saved variables, bind the return value to `into` when
present, and set the PC to `return_pc`; a top-level `Return` with no frame SHALL
set `Status::Complete` and return `Complete`. Evidenced by the `Call` and `Return`
arms of `step` and the test `func_call_resolves_return_value` in
`crates/steer-core/src/vm.rs`.

#### Scenario: a call pushes a frame and jumps to the body

- **WHEN** `step` executes a `Call` with positional arguments
- **THEN** a frame is pushed with `return_pc` set to PC+1, the caller's variables
  are saved, the new scope replaces `vars`, and the PC jumps to `entry`.

#### Scenario: a return restores the caller and binds the result

- **WHEN** `step` executes a `Return` with a value and a frame is on the stack
- **THEN** the saved variables are restored, the value is bound to `into` when
  present, and the PC is set to the frame's `return_pc`.

### Requirement: Eval errors halt with the PC stuck on the failing instruction

`step` SHALL treat an evaluation failure during a control expression as a halt:
it SHALL set `Status::Halted(<message>)` and return `StepOutcome::Error(
<EvalError>)`, and the PC SHALL remain on the failing instruction so no further
advance is possible. `report_error(ctx, reason)` SHALL set `Status::Halted(reason)`.
Evidenced by `eval_error`/`report_error` and the test `report_error_halts` in
`crates/steer-core/src/vm.rs`.

#### Scenario: an eval failure halts and pins the PC

- **WHEN** a control expression in `step` raises an `EvalError`
- **THEN** the status becomes `Halted`, `Error` is returned, and the PC stays on
  the failing instruction.

### Requirement: Status is terminal and sticky once complete or halted

`Status` SHALL be one of `Running`, `Complete`, or `Halted(String)`, and
`is_running()` SHALL be true only for `Running`. Once the status is `Complete` or
`Halted`, it is terminal: subsequent `step` and `check` calls SHALL return
`NotRunning` and SHALL NOT resume execution. Evidenced by `enum Status` /
`Context::is_running` in `crates/steer-core/src/context.rs`.

#### Scenario: a completed run cannot be advanced

- **WHEN** `step` or `check` is called on a `Complete` or `Halted` context
- **THEN** the call returns `NotRunning` and the status does not change.

### Requirement: SetMeta applies both template and context directives with empty to None

`step` SHALL, on `Instr::SetMeta`, evaluate the expression and apply the directive
by key: a `template` key SHALL set `meta.template_dir`, and a `context` key SHALL
set `meta.context`. For either key, an empty rendered value SHALL map to `None`
and a non-empty rendered value SHALL map to the rendered string; the two keys are
distinct applications, not a single shared rule. Evidenced by `apply_meta` in
`crates/steer-core/src/vm.rs`.

#### Scenario: a context directive applies empty to None

- **WHEN** `step` executes a `SetMeta` whose key is `context` and whose expression
  renders to an empty string
- **THEN** `meta.context` is set to `None`.

#### Scenario: a non-empty context directive stores the rendered text

- **WHEN** `step` executes a `SetMeta` whose key is `context` and whose expression
  renders to non-empty text
- **THEN** `meta.context` is set to that rendered string.
