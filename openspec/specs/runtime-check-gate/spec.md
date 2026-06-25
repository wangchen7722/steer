# Runtime Check Gate

## Purpose

`check` is the three-mode verification gate over the current agent operation. It
dispatches by operation kind (auto / value / checked), enforces pass-consumed-once
so a pass report cannot leak across loop iterations, rejects malformed
`checked` payloads, and renders the exact `steer instance set` report commands an
agent must run. Return-type enforcement for value operations happens at the
single `set`-time commit point, not at `check`, so that value ops (which have no
`check=` gate) cannot bypass it.

## Requirements

### Requirement: check classifies the current op into one of three kinds

`check` SHALL classify the operation at the current PC via `check_kind`: an op
with a `check` named argument is `Checked`; otherwise an op with an `into` target
is `Value(target)`; otherwise it is `Auto`. Evidenced by `check_kind` /
`CheckKind` in `crates/steer-core/src/vm.rs`.

#### Scenario: a bare agent call is Auto

- **WHEN** the current op is an agent-node call with neither `check` nor `into`
- **THEN** `check_kind` classifies it as `Auto`.

#### Scenario: an op with a check arg is Checked even if it has into

- **WHEN** the current op carries a `check` named argument
- **THEN** `check_kind` classifies it as `Checked`.

### Requirement: The check gate advances, pends, or renders by kind and state

`check` SHALL return `NotRunning` when the status is not `Running`, and `Done`
when there is no `AgentOp` at the current PC. For `Auto` it SHALL advance the PC
by one and return `Advanced`. For `Value(target)` it SHALL advance the PC and
return `Advanced` when `target` is already present in `ctx.vars`, otherwise
return `Pending` without advancing. For `Checked` it SHALL consume a stored pass
report, retain a failure for retry, or render the verification instruction, per
the pass-consumed-once and checked-payload rules below. Value ops SHALL have no
return-type verification at `check`. Evidenced by `check` / `check_instruction`
and the tests `check_auto_advances_past_print`,
`value_op_check_pending_until_set` in `crates/steer-core/src/vm.rs`.

#### Scenario: an Auto op advances immediately

- **WHEN** `check` is called on an `Auto` op while `Running`
- **THEN** the PC advances by one and `Advanced` is returned.

#### Scenario: a Value op pends until its target is set

- **WHEN** `check` is called on a `Value(target)` op whose target is not yet in
  `ctx.vars`
- **THEN** `Pending` is returned and the PC does not advance.

### Requirement: A passing checked report is consumed once per iteration

`check` SHALL, for a `Checked` op whose `ctx.steps[pc].checked` is a report with
`passed() == true`, remove that `StepState` and advance the PC by one, returning
`Advanced` (pass-consumed-once). This guarantees a pass report cannot leak across
a loop back-edge into the next iteration of the same op. Evidenced by the pass
arm of `check_instruction` and the tests `passing_check_consumes_report_single_iteration`,
`for_loop_check_requires_per_iteration_report` in `crates/steer-core/src/vm.rs`.

#### Scenario: a pass report is consumed and the op advances

- **WHEN** `check` is called on a `Checked` op whose stored report has `passed`
  true
- **THEN** the `StepState` is removed, the PC advances by one, and `Advanced` is
  returned.

#### Scenario: a checked op in a loop needs a fresh report each iteration

- **WHEN** a `Checked` op sits inside a loop body and one iteration passed and
  advanced
- **THEN** the next iteration that reaches the same op MUST supply a new pass
  report; a stale report cannot carry over.

### Requirement: A failing checked report records a reason and stays for retry

`check` SHALL, for a `Checked` op whose stored report is not a pass, extract its
`failure_reason()` (defaulting to "Verification failed without a reason."), store
that reason as `failure_reason` in the `StepState`, increment `retry_count`,
retain the `checked` report for audit, and return `Failed` without advancing the
PC. The next `step` (see
[interpreter-execution](openspec/specs/interpreter-execution/spec.md)) SHALL
re-render the instruction with retry context. Evidenced by the failure arm of
`check_instruction` / `append_retry_context` in `crates/steer-core/src/vm.rs`.

#### Scenario: a failure records a reason and stays put

- **WHEN** `check` is called on a `Checked` op whose stored report is a failure
  with a reason
- **THEN** the reason is stored as `failure_reason`, `retry_count` increments,
  the report is retained, `Failed` is returned, and the PC is unchanged.

### Requirement: The checked payload accepts true or a passed/reason object only

The `checked` value contract (what `steer set checked <value>` accepts) SHALL
accept `Value::Bool(true)` as a pass-only `CheckedReport::Bool(true)` for
back-compat. It SHALL REJECT `Value::Bool(false)` with a message directing the
agent to `{"passed":false,"reason":"..."}`. A `Value::Object` SHALL be rejected
unless it contains a boolean `passed`; for `passed:false` it SHALL require a
non-blank string `reason`. Extra keys SHALL be ignored. Any other `Value` SHALL
be rejected. Evidenced by `set_value` / `checked_report` in
`crates/steer-core/src/vm.rs` and `CheckedReport` /
`CheckedReport::passed`/`failure_reason` in `crates/steer-core/src/context.rs`.

#### Scenario: bool true is a pass

- **WHEN** `steer set checked` is given `true`
- **THEN** it is accepted as `CheckedReport::Bool(true)`.

#### Scenario: bool false is rejected with a reason-directed message

- **WHEN** `steer set checked` is given `false`
- **THEN** it is rejected and the message tells the agent to use an object with
  `passed` and `reason`.

#### Scenario: a failure object must carry a non-blank reason

- **WHEN** `steer set checked` is given `{"passed":false}` with no `reason`
- **THEN** it is rejected.

### Requirement: Return-type enforcement is enforced once at set time

`validate_set_value` SHALL be the single, unbypassable commit point for
return-type enforcement. It SHALL trigger only when the instruction at the current
PC is an `AgentOp` whose `into` equals the variable being set AND whose resolved
template has a `return_spec`. For `return: bool` (or the intrinsic `judge`) it
SHALL accept only `Value::Bool`; for `return: string` it SHALL accept only
`Value::Str`; for `return: none`, a missing return spec, or a bare call it SHALL
not enforce and SHALL accept any value. On rejection, `set_value` SHALL NOT be
called and the value SHALL NOT be stored. Setting any other variable (not the
current op's `into`) SHALL be unconstrained. Evidenced by `validate_set_value` /
`check_value_against_callee` / `set_value` and the tests
`bool_return_accepts_boolean_at_set`,
`string_return_rejects_non_string_at_set`,
`set_of_non_current_op_var_is_not_type_checked` in
`crates/steer-core/src/vm.rs`; rationale commit `ea231ae`.

#### Scenario: a bool-return op accepts only a Bool at set time

- **WHEN** `steer set <var>` stores into the current `bool`-return op's `into`
  target with a `Value::Bool`
- **THEN** the value is stored; with any non-`Bool` value it is rejected and not
  stored.

#### Scenario: a value op's return type is enforced at set, not at check

- **WHEN** a value op (no `check=`) with a `return: string` spec has its target
  set
- **THEN** enforcement happens at `set`; `check` performs no type check on it.

#### Scenario: setting an unrelated variable is never type-checked

- **WHEN** `steer set <var>` targets a variable that is not the current op's
  `into`
- **THEN** no return-type enforcement applies and any `Value` is stored.

### Requirement: Verification instructions carry the exact report commands

Every rendered verification instruction (the `Checked` `None` arm) SHALL carry a
`<steer-system-reminder>` block with exactly two commands the agent may run: a
pass form `steer instance set <instance> checked '{"passed":true}'` and a fail
form `steer instance set <instance> checked '{"passed":false,"reason":"<why it
failed>"}'`. Evidenced by `CHECK_REPORT_TEMPLATE` / `render_check_report` in
`crates/steer-core/src/template.rs` and the `None` arm of `check_instruction`.

#### Scenario: the system reminder lists pass and fail commands

- **WHEN** `check` renders a verification instruction for a `Checked` op with no
  stored report
- **THEN** the instruction text contains the pass and fail `steer instance set
  checked` commands inside a `<steer-system-reminder>` block.

### Requirement: A non-checked variable set stores the value into the instance variables

`set_value` SHALL, for any variable name other than `checked`, store the parsed
`Value` directly into the instance variables under that name and succeed, once
the return-type gate above has passed. Only the `checked` name SHALL take the
verification-payload branch. Evidenced by the non-`checked` branch of `set_value`
in `crates/steer-core/src/vm.rs`.

#### Scenario: a normal variable is stored verbatim

- **WHEN** `steer set <name> <var> <value>` targets a variable other than
  `checked`
- **THEN** the parsed value is stored into the instance variables under `<var>`
  and the command succeeds.
