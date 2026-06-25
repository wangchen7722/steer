## 1. Type-check helper

- [x] 1.1 In `crates/steer-core/src/vm.rs`, add a helper that, given the
  callee's resolved `NodeTemplate` `return_spec().kind` and the `Value` the
  agent set, returns `Ok(())` when the type matches or `Err(reason)` when it
  does not. Cover the matrix: `IntrinsicBool`/`Bool` accept only
  `Value::Bool`; `String` accepts only `Value::Str`; `None`, missing
  `return_spec`, and bare calls are unchecked (`Ok(())`).
- [x] 1.2 Produce the kind-specific reason strings: bool mismatch →
  `expected a boolean (true/false), got <variant>`; string mismatch →
  `expected a string, got an object — if you meant to report structured
  data, that's not supported by return:string`.

## 2. Wire enforcement into check

- [x] 2.1 In `check`'s `CheckKind::Value(target)` branch, after confirming the
  key exists, resolve the callee template via
  `resolve_template_with_meta(&call.callee, &ctx.meta)` and run the type
  check from 1.1 against `ctx.vars[&target]`.
- [x] 2.2 On type mismatch, return `CheckOutcome::Failed` and store the
  reason on the step via the same `ctx.steps.entry(pc)` path as the
  `CheckKind::Checked` failure branch (set `failure_reason`, increment
  `retry_count`). Do NOT advance the PC.
- [x] 2.3 On match (or unchecked kind), advance as today. On template
  resolution failure / unknown callee, fall back to existing key-presence
  behavior (no enforcement).
- [x] 2.4 Confirm `append_retry_context` surfaces the reason on the next
  `step` with no new wiring.

## 3. Tests

- [x] 3.1 Unit test: bool callee accepts `Value::Bool(true)`/`(false)` →
  `Advanced`.
- [x] 3.2 Unit test: bool callee rejects `Value::Object` (the reported bug:
  `covered` set to a JSON verdict) → `Failed` + reason, PC unchanged.
- [x] 3.3 Unit test: string callee accepts `Value::Str` → `Advanced`.
- [x] 3.4 Unit test: string callee rejects `Value::Object` → `Failed` with
  the structured-data reason.
- [x] 3.5 Unit test: `return: none` / missing `return_spec` / bare call are
  unchecked → existing `Advanced`/`Pending` behavior.
- [x] 3.6 Unit test: the retry reason from a type failure is appended to the
  next `step` instruction for the same op.

## 4. Audit and gates

- [x] 4.1 `grep` `.steer/workflows/` for `= <callee>(..., return=...)`
  assignments and confirm none rely on storing non-string JSON under
  `return: string`.
- [x] 4.2 `cargo fmt --all -- --check`
- [x] 4.3 `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] 4.4 `cargo test --workspace --all-features`
- [x] 4.5 Update the behavior specs under `docs/specs/` (runtime-check,
  instance, control-flow) to mirror the new scenarios, per the repo's
  `docs/` index convention.
