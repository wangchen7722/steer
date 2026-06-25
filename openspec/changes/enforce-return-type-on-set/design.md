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
- At `check` time for value ops, reject a value whose `Value` variant does not
  match the callee's declared `return` `ParamKind`, and drive a retry instead
  of advancing.
- Cover `IntrinsicBool`/`Bool` (must be `Value::Bool`) and `String` (must be
  `Value::Str`).
- Reuse the existing failure-reason + retry machinery so the agent gets a
  precise reason to correct itself — directly addressing the
  context-compression "forgot the command" failure mode.
- Zero CLI surface change.

**Non-Goals:**
- Hardening `set_value` itself (set time). Enforcement is at `check` time,
  where the callee's `return` type is correlatable and a retry is drivable.
  A wrong value stays stored until `check` rejects it.
- Touching the `checked` (`check=`) path or its backward-compatible
  `true`/`false`/`{passed,reason}` acceptance.
- Introducing a `return: json` / `return: object` type. Structured-data
  reporting via `return: string` remains unsupported; the rejection reason
  says so explicitly. A future change can add such a type if needed.
- Type-checking `None`, bare (unassigned) calls, or callees with no `return`
  spec — no declared type to enforce.

## Decisions

### Decision 1: Enforce at `check` time, not `set` time

**Choice:** Type enforcement lives in `check`'s `CheckKind::Value` branch.

**Rationale:** `set_value` is a low-level primitive called from `steer
instance set` with no knowledge of which callee the variable belongs to — the
callee is an IR/VM concept known only to the op at `ctx.pc`. `check` already
holds `Instr::AgentOp { call, into }` and resolves the callee's template, so
it is the natural and only place that can correlate value ↔ declared type.
Enforcing at `check` time also lets the engine *retry* (return `Failed` +
reason), which a `set`-time rejection could not do cleanly (there is no op
context to attach a retry reason to).

**Alternative considered:** Harden `set_value` to take a declared type.
Rejected: `set` is called by the CLI with just `(name, var, value)`; threading
the callee's `return` type down to it would require the CLI to look up the
current op, duplicating VM state in the CLI and breaking the "VM is the only
state authority" invariant. It also couldn't drive a retry.

### Decision 2: Reject via `CheckOutcome::Failed` + failure-reason, mirroring the `checked` path

**Choice:** A type mismatch returns `CheckOutcome::Failed` and stores a
`failure_reason` on the step (via the same `ctx.steps.entry(pc)` path used by
the `checked` failure case), incrementing `retry_count`. The next `step`
appends the reason through the existing `append_retry_context`.

**Rationale:** This is the exact mechanism already used for `check=` failures
(vm.rs `CheckKind::Checked` failure branch). Reusing it means: zero new
outcome variants, the CLI needs no change (`Failed` → `"failed"` already), and
the agent-facing retry loop is identical. The failure reason text is the
agent's signal to re-issue the correct `set`.

**Alternative considered:** A new `CheckOutcome::TypeError`. Rejected — it
would be observably identical to `Failed` from the CLI/agent side and just
adds a variant to maintain.

**Alternative considered:** `Status::Halted`. Rejected — too destructive for a
recoverable agent mistake; wastes the tokens already spent on the run.

### Decision 3: Type matrix keyed on `ParamKind`

**Choice:**

| `return` `ParamKind` | Accepted `Value` variant | On mismatch |
|---|---|---|
| `IntrinsicBool`, `Bool` | `Value::Bool` | `Failed` + reason |
| `String` | `Value::Str` | `Failed` + reason |
| `None` / no `return` spec / bare call | (unchecked) | existing behavior |

`None` and bare calls have no declared type, so no enforcement applies — they
keep the current key-presence / `Auto` semantics.

**Rationale:** `IntrinsicBool` is the live bug (`judge`); `String` is the
common case (`task`/`collect`/`ask`/`command`). `None` (`print`) and bare
calls have no `into` receiver that the agent sets, so they never reach the
type-check. The matrix is exhaustive over the declared-type space.

### Decision 4: Reason strings are specific per kind

**Choice:**
- Bool mismatch: `expected a boolean (true/false), got <kind>`
- String mismatch: `expected a string, got an object — if you meant to report
  structured data, that's not supported by return:string`

where `<kind>` names the actual variant (object / list / number / string).

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

- **[Risk] `return: string` workflows that today store JSON objects will start
  failing `check`.**
  → Mitigation: This was always an unsupported use (the value was stored but
  never a real string); the rejection reason guides the author. The
  `openspec-generate-specs` workflow uses `return: string` only for genuine
  string results (`collect`'s `PRIOR`/`BOOTSTRAP`), so the blast radius is
  small. Audit existing `return="..."` assignments before merge.

- **[Risk] Extra retry round costs tokens on a type mismatch.**
  → Mitigation: This is the intended behavior — a silent wrong result costs
  more (masked gaps) than one corrective retry. The retry reason is precise,
  so a single retry typically suffices.

- **[Risk] `retry_count` is currently unbounded.**
  → Mitigation: Pre-existing condition (the `checked` path has the same
  property). Not introduced by this change; if a bound is wanted it is a
  separate change.

- **[Trade-off] Enforcement is at `check` time, so a wrong value is briefly
  stored in `ctx.vars`.**
  → Accepted: between `set` and `check` the wrong value exists, but it cannot
  affect control flow until `check` advances the op, and `check` is the gate
  that consumes it. No `step` past the op happens without `check` succeeding.

## Migration Plan

1. Implement the `CheckKind::Value` type check + reasons.
2. Add unit tests for each matrix cell (bool-accept, bool-reject,
  string-accept, string-reject, none-unchecked, bare-unchecked,
  no-spec-unchecked).
3. `grep` the repo's `.steer/workflows/` for `= <callee>(..., return=...)`
  assignments and confirm none rely on storing non-string JSON under
  `return: string`.
4. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features`.

**Rollback:** Revert the `CheckKind::Value` branch to key-presence-only; no
data-format or persistence change is involved (the stored `Value`s are
unchanged, only the `check` decision changes).

## Open Questions

- Should a `retry_count` cap eventually halt a run stuck on repeated type
  mismatches? Out of scope here; tracked as a possible follow-up.
