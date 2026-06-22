# 6. Control Flow

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 6.1 Prefer early returns

Use guard clauses to keep the main path shallow. Handle the "nothing to do" case before touching
the main data.

```rust
// Correct
pub fn step(ir: &[Instr], ctx: &mut Context) -> StepOutcome {
    if !ctx.is_running() {
        return StepOutcome::NotRunning;
    }
    loop {
        let Some(instr) = ir.get(ctx.pc as usize) else {
            ctx.status = Status::Complete;
            return StepOutcome::Complete;
        };
        match instr {
            // ... main dispatch ...
        }
    }
}
```

## 6.2 Match exhaustively

Use exhaustive `match` for domain enums when each variant has distinct semantics. List every
variant explicitly and, for arms that are impossible *by construction*, name the reason with
`unreachable!(...)` rather than hiding them behind `_`.

```rust
// Correct: every BinOp variant is listed.
fn apply_binop(op: BinOp, l: &Value, r: &Value) -> Result<Value, EvalError> {
    use BinOp::*;
    match op {
        Add | Sub | Mul | Div => { /* arithmetic */ }
        Eq => Ok(Value::Bool(values_eq(l, r))),
        Ne => Ok(Value::Bool(!values_eq(l, r))),
        Lt | Gt | Le | Ge => { /* comparison */ }
        // Logical operators are short-circuit evaluated in `eval` and never reach here.
        And | Or => unreachable!("logical operators are short-circuit evaluated"),
    }
}
```

Do not use `_` to hide unhandled variants in core domain logic.

```rust
// Incorrect: adding a new Instr variant may silently take the wrong path.
match instr {
    Instr::AgentOp { .. } => render,
    _ => StepOutcome::NotRunning,
}
```

Use `if let` or `let ... else` when exactly one pattern matters.

```rust
// Correct: the else branch completes the run when the PC runs off the end.
let Some(instr) = ir.get(ctx.pc as usize) else {
    ctx.status = Status::Complete;
    return StepOutcome::Complete;
};
```

## 6.3 Keep boolean expressions readable

Name complex predicates instead of inlining them repeatedly.

```rust
// Incorrect
if !ctx.is_running() && attempts < max && last.is_retryable() {
    // ...
}

// Correct
let may_retry = !ctx.is_running() && attempts < max && last.is_retryable();
if may_retry {
    // ...
}
```

Prefer expressing such conditions as a predicate method so the call site reads as a question —
e.g. `ctx.is_running()` or `value.truthy()`. Use parentheses only when they make precedence
materially clearer.
