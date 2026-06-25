## Context

steer drives agents through an instruction-step VM. An *assigned value op*
(`x = judge("...")`, `x = task("...", return="...")`) renders an instruction
telling the agent which variable to `steer instance set`. The VM's `check`
function then advances past the op once the agent has set the value.

The current `check` logic for value ops is in
`crates/steer-core/src/vm.rs`, `CheckKind::Value(target)`:

```rust
CheckKind::Value(target) => {
    if ctx.vars.contains_key(&target) {
        ctx.pc += 1;
        CheckOutcome::Advanced
    } else {
        CheckOutcome::Pending
    }
}
```

It tests only *key presence*. `set_value` stores whatever the agent sent
(`parse_value` turns valid JSON into typed `Value`s — an object becomes
`Value::Object`). Nothing correlates the stored value against the callee's
declared `return` type. Because `truthy()` treats any non-empty object as
true, a wrong-typed value silently passes conditions (`until covered`).

The callee's declared `return` type IS already known to the engine:
`resolve_template_with_meta(&call.callee, &ctx.meta)` returns a
`NodeTemplate` whose `return_spec()` yields a `ParamSpec { kind: ParamKind }`
where `kind` is `IntrinsicBool` (`return: bool`), `Bool`, `String`, or `None`.

The `checked` special variable (the `check=` path) already has its own
structural validation in `checked_report` — `true` or
`{"passed":bool,"reason":"..."}`. That path is a separate `CheckKind` and is
explicitly out of scope.

## Goals / Non-Goals

**Goals:**
- At `set` time, reject a value whose `Value` variant does not match the
  callee's declared `return` `ParamKind`, when the variable being set is the
  current op's assignment target. The value is never stored on mismatch.
- Cover `IntrinsicBool`/`Bool` (must be `Value::Bool`) and `String` (must be
  `Value::Str`).
- Reject with a precise, agent-facing reason so the agent can correct itself —
  directly addressing the context-compression "forgot the command" failure
  mode.
- The `checked` (`check=`) path keeps its own structural validation and is
  unaffected.

**Non-Goals:**
- Enforcing at `check` time. An earlier version of this change enforced at
  `check`, but a value op has no `check=` gate forcing the agent to call
  `check` — the agent can `set` and then `step` past the op without ever
  checking, so a `check`-time gate is bypassable. `set` time is the single,
  unbypassable enforcement point. (`check`'s value-op branch stays pure
  key-presence, as before.)
- Touching the `checked` (`check=`) path or its backward-compatible
  `true`/`false`/`{passed,reason}` acceptance.
- Introducing a `return: json` / `return: object` type. Structured-data
  reporting via `return: string` remains unsupported; the rejection reason
  says so explicitly. A future change can add such a type if needed.
- Type-checking `None`, bare (unassigned) calls, or callees with no `return`
  spec — no declared type to enforce.
- Type-checking `set` of variables that are NOT the current op's target (a
  workflow-assigned local, an already-completed op's variable). Those have no
  active callee return type to check against.

## Decisions

### Decision 1: Enforce at `set` time, not `check` time

**Choice:** Type enforcement lives in `validate_set_value`, called from the
CLI's `run_instance_set` before `set_value` stores anything. It uses
`ir[ctx.pc]` (the current op) to find the callee and its declared `return`
type, and only enforces when the variable being set equals the op's `into`.

**Rationale:** The agent's interaction with a value op is `step` → `set` →
(possibly) `check`/`step`. A value op has no `check=` clause, so nothing
forces the agent to call `check` before advancing — the agent can `set` a
wrong value and then `step` past the op, never hitting a `check`-time gate.
Enforcing at `set` time is the only unbypassable point: every value an agent
commits passes through `steer instance set`. It also keeps a wrong value from
ever being stored, so no downstream `until`/`if` can see it. The CLI's
`with_instance_result` closure already receives `(ir, ctx, name)`, so
`ir[ctx.pc]` is available without threading new state.

**Alternative considered:** Enforce at `check` time (in `CheckKind::Value`).
Rejected after the user reproduced the bypass: `bug_slug = ask(...,
return="bug slug in kebab-case")` followed by `set ... bug_slug false` and a
direct `step` skipped `check` entirely. A `check`-time gate only works for
the `check=` path, which already has its own `checked` validation.

**Alternative considered:** Harden the core `set_value` primitive itself.
Rejected: `set_value(ctx, var, value)` has no IR and no notion of a current
op; making it type-aware would force every caller (including internal
`Assign`) to supply callee context it doesn't have. Keeping the policy in a
dedicated `validate_set_value(ir, ctx, var, value)` that the CLI calls before
`set_value` preserves the primitive's simplicity.

### Decision 2: Reject via `Err(reason)` from `set`, printed to the agent

**Choice:** `validate_set_value` returns `Err(reason)` on a type mismatch.
`run_instance_set` propagates it through `with_instance_result`, which prints
`error: <reason>` to stderr and exits non-zero, **without storing the value**.
The agent sees the reason on its `steer instance set` call and re-issues the
correct one.

**Rationale:** `set` is the agent's single commit point for a value; rejecting
there with a precise reason is the most direct signal. No new VM outcome
variant, no retry-state machinery, and the wrong value never reaches
`ctx.vars`. The `check=` path keeps its own `CheckOutcome::Failed` + retry
flow unchanged.

**Alternative considered:** Reject at `check` via `CheckOutcome::Failed` +
`failure_reason` + `retry_count` (the `checked`-path mechanism). Rejected: it
is bypassable when the agent skips `check` (see Decision 1), and it lets the
wrong value sit in `ctx.vars` between `set` and `check`.

**Alternative considered:** `Status::Halted`. Rejected — too destructive for a
recoverable agent mistake; wastes the tokens already spent on the run.

### Decision 3: Type matrix keyed on `ParamKind`

**Choice:**

| `return` `ParamKind` | Accepted `Value` variant | On mismatch |
|---|---|---|
| `IntrinsicBool`, `Bool` | `Value::Bool` | rejected at `set` with reason |
| `String` | `Value::Str` | rejected at `set` with reason |
| `None` / no `return` spec / bare call | (unchecked) | accepted (no declared type) |

`None` and bare calls have no declared type, so no enforcement applies.

**Rationale:** `IntrinsicBool` is the live bug (`judge`); `String` is the
common case (`task`/`collect`/`ask`/`command`). `None` (`print`) and bare
calls have no `into` receiver that the agent sets, so they never reach the
type-check. The matrix is exhaustive over the declared-type space.

### Decision 4: Reason strings are specific per kind

**Choice:**
- Bool mismatch: `expected a boolean (true/false) for \`<callee>\`, got <kind>`
- String mismatch: `expected a string for \`<callee>\`, got <kind> — if you
  meant to report structured data, that's not supported by return:string`

where `<kind>` names the actual variant (object / boolean / number / list /
string). Both string-mismatch cases (object or any other non-string variant)
carry the `return:string` guidance.

**Rationale:** A precise reason is the whole point — the agent forgot the
command; the reason must tell it exactly what to set. The String reason
explicitly closes the escape hatch so a structured-data reporter isn't left
guessing.

### Decision 5: Resolution failure is non-fatal

**Choice:** If `resolve_template_with_meta` cannot find a template for the
callee (unknown callee, generic fallback with no `return` spec), enforcement
is skipped — fall back to existing key-presence behavior.

**Rationale:** `validate` accepts unknown callees (per the project's
template-mechanism stance), and the generic fallback carries no `return` spec.
Failing the run because a callee's type is undeclared would be a regression
for workflows that rely on bare custom callees.

## Risks / Trade-offs

- **[Risk] `return: string` workflows that today store JSON objects via `set`
  will start being rejected.**
  → Mitigation: This was always an unsupported use (the value was stored but
  never a real string); the rejection reason guides the author. The
  `openspec-generate-specs` workflow uses `return: string` only for genuine
  string results (`collect`'s `PRIOR`/`BOOTSTRAP`), so the blast radius is
  small. Audit existing `return="..."` assignments before merge.

- **[Risk] A rejected `set` costs the agent a corrective retry.**
  → Mitigation: This is the intended behavior — a silent wrong result costs
  more (masked gaps) than one corrective `set`. The reason is precise, so a
  single retry typically suffices.

- **[Trade-off] Only the current op's target variable is type-checked at `set`.**
  → Accepted: a `set` of any other variable (a workflow local, an
  already-completed op's variable) has no active callee return type to check
  against, so it is allowed. This is the correct scope — the type contract
  belongs to the op currently awaiting a value.

## Migration Plan

1. Implement `validate_set_value` + `check_value_against_callee` and the
  kind-specific reasons.
2. Wire `run_instance_set` to call `validate_set_value` before `set_value`.
4. Add unit tests for each matrix cell (bool-accept, bool-reject,
  string-accept, string-reject-object, string-reject-non-string,
  non-target-var-unchecked, bare-callee-unchecked).
5. `grep` the repo's `.steer/workflows/` for `= <callee>(..., return=...)`
  assignments and confirm none rely on storing non-string JSON under
  `return: string`.
6. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features`.
7. End-to-end check: `steer instance start <wf> <name>`, `step` to a value op,
  `set <name> <var> <wrong-type>` must fail with a reason and not store;
  `set` with a correct type must succeed.

**Rollback:** Remove the `validate_set_value` call from `run_instance_set`
(and the helper if unused); `check`'s value-op branch is already unchanged
from before this change, so there is nothing to revert there. No data-format
or persistence change is involved.

## Open Questions

- None. (An earlier open question about a `retry_count` cap no longer applies:
  `set`-time rejection does not touch retry state.)
