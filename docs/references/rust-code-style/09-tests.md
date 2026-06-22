# 9. Tests

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 9.1 Test behavior, not implementation details

Test names SHOULD describe the observed behavior and condition.

```rust
// Weak
#[test]
fn test_value() {}
#[test]
fn test_parse() {}

// Correct
#[test]
fn eval_unset_var_is_error() {}
#[test]
fn parse_value_typed_literals() {}
#[test]
fn context_round_trips_through_json() {}
```

Tests SHOULD use Arrange–Act–Assert structure with visually separated phases.

```rust
// Correct
#[test]
fn context_round_trips_through_json() {
    // Arrange
    let mut c = Context::new();
    c.pc = 3;
    c.vars.insert("x".into(), Value::Int(5));
    c.steps.insert(2, StepState { checked: Some(true), attempts: 1 });

    // Act
    let json = serde_json::to_string(&c).unwrap();
    let back: Context = serde_json::from_str(&json).unwrap();

    // Assert
    assert_eq!(c, back);
}
```

## 9.2 Prefer semantic assertions

Assert on the observable result, not on private representation unless representation *is* the
contract. Assert on the returned `Value` / `Result`, not on intermediate state.

```rust
// Correct: the assertion is about the evaluated value.
#[test]
fn eval_arithmetic() {
    assert_eq!(
        eval(&expr_of("1 + 2 * 3"), &HashMap::new()),
        Ok(Value::Int(7))
    );
    assert_eq!(eval(&expr_of("7 / 2"), &HashMap::new()), Ok(Value::Int(3)));
}

// Correct: the error variant and its payload, not a string match.
#[test]
fn eval_unset_var_is_error() {
    assert_eq!(
        eval(&expr_of("missing"), &HashMap::new()),
        Err(EvalError::UnsetVar("missing".into()))
    );
}
```

## 9.3 Test boundaries

Unit tests live inline (`#[cfg(test)] mod tests`) next to the code they cover. Place them so each
module's tests exercise its own concern: parsing and AST spans in the syntax crate, evaluation and
lowering in the core crate, serialization round-trips on the persisted types, and so on.

Integration tests live under `crates/<binary>/tests/` and drive the compiled binary through
`std::process::Command`, covering the user-facing commands end to end.

Prefer fakes or test doubles over real external dependencies. When an external system drives the
program (an agent, a network service, a subprocess), substitute canned inputs so tests never
depend on a live system, network, credentials, or nondeterministic output.

```rust
// Correct: an integration test runs the binary and feeds it canned values.
let path = write_tmp("valid", "task(\"do something\", return=\"path\")\n");
let out = run_cli().args(["workflow", "validate"]).arg(&path).output().expect("run cli");
assert!(out.status.success(), "expected success");
```
