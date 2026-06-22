# 5. Error Handling

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 5.1 Preserve context at boundaries

Errors MUST retain enough context to diagnose the failed operation. Capture the underlying cause
at each subsystem boundary (io, serde, parsing, ...) using thiserror `#[from]`, so the original
error is never stringified away.

```rust
// Correct
#[derive(Debug, Error)]
pub enum InstanceError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("invalid context.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("workflow did not parse: {0}")]
    Parse(#[from] ParseError),
}
```

Add context at subsystem boundaries, such as:

- CLI argument parsing;
- filesystem and process boundaries (`start_instance`, `load_ir`, `save_context`);
- parse and validation boundaries (`parse`, `validate`);
- template or format parsing.

Do not repeatedly wrap the same error at every internal function boundary.

```rust
// Incorrect: noisy, duplicate context that throws away the structured error.
fn load_context(dir: &Path) -> Result<Context, InstanceError> {
    let src = fs::read_to_string(dir.join(CONTEXT_FILE))
        .map_err(InstanceError::Io)?; // already covered by #[from]
    // ...
}

// Correct: let #[from] and `?` carry the context once.
fn load_context(dir: &Path) -> Result<Context, InstanceError> {
    let src = fs::read_to_string(dir.join(CONTEXT_FILE))?;
    Ok(serde_json::from_str(&src)?)
}
```

## 5.2 Do not panic for recoverable failures

Production code MUST NOT use `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, or
`unimplemented!()` for input errors, I/O failures, malformed input, or normal runtime states.
(Deny the `todo`/`unimplemented` lints at the workspace level so they cannot slip in.)

```rust
// Incorrect
let module = parse(&src).unwrap();

// Correct
let module = parse(&src)?;
```

`.unwrap()` and `.expect()` are acceptable in tests. They are also acceptable for a local
invariant only when the invariant is genuinely impossible to violate in the current code path.

```rust
// Correct in a test.
let json = serde_json::to_string(&c).unwrap();
let back: Context = serde_json::from_str(&json).unwrap();
```

When `expect` is justified in production code, the message MUST state the violated invariant.

```rust
// Correct: the reason is stated.
let tmpl = Template::parse(body).expect("built-in templates must parse");

match op {
    Add => x.checked_add(y),
    // ...
    _ => unreachable!("narrowed by the outer match arm"),
}
```

`.unwrap_or(…)` is acceptable when a fallback is the intended behavior, not error-hiding:

```rust
// Correct: 0.0 is the intended value for a non-finite JSON number.
Value::Float(n.as_f64().unwrap_or(0.0))
```

## 5.3 Error messages

Human-facing error messages SHOULD:

- start with lowercase;
- avoid trailing punctuation;
- describe the failed operation;
- include relevant identifiers, paths, or the failing input;
- avoid exposing secrets or full environment contents.

```rust
// Incorrect
"Error: Failed!"

// Correct
"variable `gate` is not set"
"type error: expected a number, found null"
"invalid context.json: expected ident at line 1 column 3"
"workflow did not parse: unexpected token at line 12, col 4"
```
