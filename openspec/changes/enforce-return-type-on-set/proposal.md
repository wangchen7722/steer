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

- Enforce the callee's declared `return` type at `set` time for value ops
  (assigned calls without `check=`, e.g. `covered = judge(...)`). When the
  value an agent sets does not match the declared `return` kind, `set` is
  rejected with a clear reason and the value is **not stored** — the agent
  re-issues the correct `steer instance set`.
- Type-checking matrix, keyed on the `return` parameter's `ParamKind`:
  - `IntrinsicBool` (and `Bool`): only `Value::Bool` is accepted.
  - `String`: only `Value::Str` is accepted. On rejection the reason states:
    `expected a string, got <kind> — if you meant to report structured data,
    that's not supported by return:string`.
  - `None`, no `return` spec, or a bare (unassigned) call: not type-checked
    (no declared type to enforce; existing behavior preserved).
- Enforcement applies only when the variable being set is the current op's
  assignment target (`into`). Setting any other variable is unconstrained.
- The `checked` special variable (`check=` path) is **unchanged**. It still
  accepts both `true`/`false` and `{"passed":bool,"reason":"..."}` for
  backward compatibility — this change does not touch `checked_report`.
- `set_value` itself (the core primitive) is **not** hardened; a dedicated
  `validate_set_value(ir, ctx, var, value)` is called by the CLI's
  `run_instance_set` before `set_value`, so the primitive stays simple. `check`'s
  value-op branch keeps its existing key-presence behavior (a value op has no
  `check=` gate, so `check`-time enforcement would be bypassable).

## Capabilities

### New Capabilities
<!-- None. This hardens existing VM/instance behavior; no new capability. -->

### Modified Capabilities

This repo records behavior specs under `docs/specs/` (not `openspec/specs/`,
which is empty). The affected behavior layers:

- `runtime-check`: The value-op `check` branch is **unchanged** (pure
  key-presence); type enforcement is not at `check` for value ops. A new
  requirement covers set-time enforcement.
- `instance`: The `set writes typed values` scenario is refined — `set` now
  enforces the current op's declared `return` type and rejects (without
  storing) a wrong-typed value, instead of accepting any parsed value.
- `control-flow`: The `judge` returns-a-boolean scenario is strengthened — a
  non-bool value set by the agent is rejected at `set`, so `until covered`
  cannot be fooled by a truthy object.

## Impact

- **Code**: `crates/steer-core/src/vm.rs` (new `validate_set_value` +
  `check_value_against_callee`), assisted by
  `crates/steer-core/src/template.rs` (`resolve_template_with_meta` +
  `NodeTemplate::return_spec`); `crates/steer-cli/src/main.rs`
  (`run_instance_set` calls `validate_set_value` before `set_value`). The
  `checked` path (`checked_report`) and `check`'s value-op branch are
  untouched.
- **APIs/CLI**: `steer instance set` now exits non-zero with `error: <reason>`
  on a type mismatch (value not stored). No new subcommands or flags.
- **Behavior**: Value-op `set`s with a wrong type are rejected immediately.
  Workflows that already set correct types are unaffected. Workflows
  relying on storing structured JSON via `return: string` (an unsupported use
  today) will see a clear rejection with guidance.
- **Tests**: New unit tests in `vm.rs` for each branch of the type matrix;
  the existing value-op scenarios stay green for correct types.
