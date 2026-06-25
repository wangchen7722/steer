# Instance Lifecycle

## Purpose

Defines the run-state machine an instance obeys across CLI invocations: a
`Context` starts `Running` at program counter zero and advances toward a
terminal `Complete` or `Halted(reason)` state that sticks until a fresh
`start` resets it.

## Requirements

### Requirement: A fresh start initializes Running at program counter zero

`start_instance` SHALL initialize every new instance with `pc = 0`,
`status = Running`, empty `vars`/`frames`/`steps`, and default `meta`. A run
is considered active (`is_running()`) if and only if `status == Running`.
Evidenced by `Context::new` / `is_running` in `crates/steer-core/src/context.rs`
and the test `new_context_starts_running_at_zero`.

#### Scenario: a new context is running at zero

- **WHEN** `start_instance` creates a fresh context for a workflow
- **THEN** the context has `status == Running`, `pc == 0`, and empty
  `vars`/`frames`/`steps`.

#### Scenario: start surfaces a declared context banner

- **WHEN** the workflow declares `@context = "..."` (non-empty) and the user
  runs `steer instance start`
- **THEN** start returns the rendered context string as `Ok(Some(desc))`.

#### Scenario: start returns none without a context directive

- **WHEN** the workflow has no `@context` directive (or it renders empty) and
  the user runs `steer instance start`
- **THEN** start returns `Ok(None)`.

### Requirement: Status renders as running, complete, or halted with reason

The CLI status command SHALL render `Running` as `running`, `Complete` as
`complete`, and `Halted(reason)` as `halted: <reason>`. Evidenced by the
status rendering in `crates/steer-cli/src/main.rs`.

#### Scenario: status strings

- **WHEN** the user runs `steer instance status` on an instance in each state
- **THEN** a `Running` instance prints `running`, a `Complete` instance prints
  `complete`, and a `Halted(r)` instance prints `halted: <r>`.

### Requirement: Transitions into Complete are reached by step only

`step` SHALL transition the run to `Complete` when it runs off the end of the
instruction list (`ir.get(pc)` is `None`), when it executes `Instr::Halt`, or
when it executes a top-level `Instr::Return` with an empty frame stack.
Evidenced by the step loop in `crates/steer-core/src/vm.rs`.

#### Scenario: running off the end completes

- **WHEN** `step` is invoked and `pc` is past the last instruction
- **THEN** the status becomes `Complete`.

#### Scenario: an explicit halt instruction completes

- **WHEN** `step` executes an `Instr::Halt`
- **THEN** the status becomes `Complete`.

#### Scenario: a top-level return completes

- **WHEN** `step` executes a top-level `Instr::Return` with an empty frame
  stack
- **THEN** the status becomes `Complete`.

### Requirement: Errors during step or an explicit error command halt the run

The run SHALL halt with a reason when `step` raises a fatal `EvalError`
(unset variable, type error, or arithmetic error), recording the error's
`Display` message as the reason; the `steer instance error <name> <reason>`
command SHALL likewise halt the run with the given reason verbatim. Evidenced
by `eval_error` and `report_error` in `crates/steer-core/src/vm.rs`.

#### Scenario: an eval error halts with its message

- **WHEN** `step` raises an `EvalError` (e.g. referencing an unset variable)
- **THEN** the status becomes `Halted(<error display message>)`.

#### Scenario: the error command records a reason

- **WHEN** the user runs `steer instance error <name> <reason>`
- **THEN** the status becomes `Halted(<reason>)` with the reason joined
  verbatim.

### Requirement: Terminal states are sticky until a fresh start

Once `Complete` or `Halted`, the run SHALL remain terminal: subsequent `step`
invocations SHALL return `StepOutcome::NotRunning` without advancing, and
subsequent `check` invocations SHALL return `CheckOutcome::NotRunning` without
evaluating. Terminal status SHALL only be cleared by a new `start_instance`
that reinitializes the context. Evidenced by the `!ctx.is_running()` guards
at the top of `step` and `check` in `crates/steer-core/src/vm.rs` and the test
`run_persists_across_load_save_cycles`. The companion
[instance-persistence](openspec/specs/instance-persistence/spec.md) layer
carries this state across CLI invocations.

#### Scenario: step refuses on a terminal run

- **WHEN** the user runs `steer instance step` on an instance whose status is
  `Complete` or `Halted`
- **THEN** step returns `NotRunning` and does not advance `pc`.

#### Scenario: check refuses on a terminal run

- **WHEN** the user runs `steer instance check` on an instance whose status is
  `Complete` or `Halted`
- **THEN** check returns `NotRunning` and performs no evaluation.

#### Scenario: a new start clears a terminal status

- **WHEN** `start_instance` is re-run on an existing terminal instance name
- **THEN** the status resets to `Running` with `pc == 0`.
