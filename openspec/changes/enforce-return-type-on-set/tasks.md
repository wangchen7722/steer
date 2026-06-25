## 1. Type-check helper

- [x] 1.1 In `crates/steer-core/src/vm.rs`, add `check_value_against_callee`
  that, given the callee's resolved `NodeTemplate` `return_spec().kind` and the
  `Value` the agent set, returns `Ok(())` when the type matches or
  `Err(reason)` when it does not. Cover the matrix: `IntrinsicBool`/`Bool`
  accept only `Value::Bool`; `String` accepts only `Value::Str`; `None`,
  missing `return_spec`, and bare calls are unchecked (`Ok(())`).
- [x] 1.2 Add `validate_set_value(ir, ctx, var, value)` that runs the check
  only when `var` equals `into` of the `AgentOp` at `ctx.pc`; otherwise
  `Ok(())`. Export it from `steer-core`.
- [x] 1.3 Produce the kind-specific reason strings: bool mismatch →
  `expected a boolean (true/false) for \`<callee>\`, got <variant>`; string
  mismatch → `expected a string for \`<callee>\`, got <variant> — if you meant
  to report structured data, that's not supported by return:string`.

## 2. Wire enforcement into set

- [x] 2.1 In `crates/steer-cli/src/main.rs`, `run_instance_set` calls
  `validate_set_value(ir, ctx, var, &parsed)` before `set_value`. On `Err`,
  `with_instance_result` prints `error: <reason>` and exits non-zero without
  storing.
- [x] 2.2 `check`'s `CheckKind::Value` branch is unchanged (pure key-presence);
  type enforcement for value ops is at `set`, not `check`.

## 3. Tests

- [x] 3.1 Unit test: bool callee accepts `Value::Bool(true)`/`(false)` at set.
- [x] 3.2 Unit test: bool callee rejects `Value::Object` (the reported bug:
  `covered` set to a JSON verdict) → `Err` with "boolean", value not stored.
- [x] 3.3 Unit test: string callee accepts `Value::Str` at set.
- [x] 3.4 Unit test: string callee rejects `Value::Bool(false)` (the user's
  `bug_slug` repro) and `Value::Object` → `Err` with "string" +
  "return:string".
- [x] 3.5 Unit test: `return: none` / missing `return_spec` / bare call are
  unchecked → `Ok(())`.
- [x] 3.6 Unit test: `set` of a non-target variable is not type-checked.

## 4. Audit and gates

- [x] 4.1 `grep` `.steer/workflows/` for `= <callee>(..., return=...)`
  assignments and confirm none rely on storing non-string JSON under
  `return: string`.
- [x] 4.2 `cargo fmt --all -- --check`
- [x] 4.3 `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] 4.4 `cargo test --workspace --all-features`
- [x] 4.5 End-to-end: `steer instance set <name> <var> <wrong-type>` rejected
  with reason and not stored; correct type accepted.
- [x] 4.6 Update the behavior specs under `docs/specs/` (runtime-check,
  instance, control-flow) to mirror the set-time scenarios, per the repo's
  `docs/` index convention.
