## Why

A long-running workflow can trigger agent context compression that makes the
agent forget the exact `steer instance set` command it was supposed to issue.
In `openspec-generate-specs`, after a complex `review` step the agent should
run `judge` and set the bool variable `covered` to `true`/`false`, but instead
it set `covered` to a JSON verdict object:

```
steer instance set gen-specs covered "{\"verdict\":\"COVERED\",...}"
```

`set_value` performs no type checking on ordinary variables — it stores the
object verbatim — and `check`'s value-op branch only tests whether the key
exists. Because `truthy()` treats any non-empty object as true, the loop's
`until covered` silently exits with a false "covered" verdict, masking
uncovered gaps. Steer must be the last line of defense: the engine, not the
agent's memory, must reject a value whose type does not match the callee's
declared `return` type.

## What Changes

- Enforce the callee's declared `return` type at `check` time for value ops
  (assigned calls without `check=`, e.g. `covered = judge(...)`). When the
  value an agent sets does not match the declared `return` kind, the op fails
  with a clear reason and the agent is asked to retry — reusing the existing
  `CheckOutcome::Failed` + failure-reason + retry machinery.
- Type-checking matrix, keyed on the `return` parameter's `ParamKind`:
  - `IntrinsicBool` (and `Bool`): only `Value::Bool` is accepted.
  - `String`: only `Value::Str` is accepted. On rejection the retry reason
    states: `expected a string, got an object — if you meant to report
    structured data, that's not supported by return:string`.
  - `None`, no `return` spec, or a bare (unassigned) call: not type-checked
    (no declared type to enforce; existing behavior preserved).
- The `checked` special variable (`check=` path) is **unchanged**. It still
  accepts both `true`/`false` and `{"passed":bool,"reason":"..."}` for
  backward compatibility — this change does not touch `checked_report`.
- `set_value` itself is **not** hardened at set time; enforcement happens at
  `check` time, where the engine can correlate the value against the callee's
  declared `return` type and drive a retry. A wrong value remains stored until
  the next `check` rejects it.

## Capabilities

### New Capabilities
<!-- None. This hardens existing VM/instance behavior; no new capability. -->

### Modified Capabilities

This repo records behavior specs under `docs/specs/` (not `openspec/specs/`,
which is empty). The affected behavior layers:

- `runtime-check`: The value-op `check` branch gains type enforcement. A
  value whose `Value` variant does not match the callee's declared `return`
  `ParamKind` yields `Failed` with a reason, instead of advancing on mere
  key-presence.
- `instance`: The `set writes typed values` scenario is refined — `set`
  still parses JSON to typed values (unchanged), but a value that will later
  fail `check`'s return-type enforcement is now a recoverable retry condition,
  not a silent success.
- `control-flow`: The `judge` returns-a-boolean scenario is strengthened — a
  non-bool value set by the agent is rejected, so `until covered` cannot be
  fooled by a truthy object.

## Impact

- **Code**: `crates/steer-core/src/vm.rs` (`check`'s `CheckKind::Value`
  branch), assisted by `crates/steer-core/src/template.rs`
  (`resolve_template_with_meta` + `NodeTemplate::return_spec`). The `checked`
  path (`checked_report`) is untouched.
- **APIs/CLI**: No CLI surface change. `CheckOutcome::Failed` is already
  mapped to `"failed"` by the CLI; the agent-facing retry loop is unchanged.
- **Behavior**: Value ops that previously advanced on a wrong-typed value now
  retry. Workflows that already set correct types are unaffected. Workflows
  relying on storing structured JSON via `return: string` (an unsupported use
  today) will see a clear rejection with guidance.
- **Tests**: New unit tests in `vm.rs` for each branch of the type matrix;
  the existing value-op scenarios stay green for correct types.
