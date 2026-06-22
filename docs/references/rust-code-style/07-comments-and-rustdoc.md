# 7. Comments and Rustdoc

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

## 7.1 Explain why, not what

Comments MUST explain non-obvious rationale, constraints, invariants, or trade-offs. Do not narrate
syntax.

```rust
// Incorrect
ctx.pc += 1; // Increment the program counter.

// Correct
// Advance past the current op; the next step resumes at the following instruction.
ctx.pc += 1;
```

```rust
// Incorrect
// Create a new map.
let mut vars = HashMap::new();

// Correct
// A plain HashMap is fine here: this scope is not persisted, so ordering does not matter.
let mut vars = HashMap::new();
```

## 7.2 Write direct sentences

Comments SHOULD use complete, direct sentences. Information that affects behavior, correctness,
ordering, or failure handling MUST appear in the main sentence rather than being hidden in
parentheses.

```rust
// Incorrect
// Short-circuits (when the LHS already decides the result).

// Correct
// Short-circuit logical operators: the RHS is only evaluated when the LHS does not
// already decide the result.
```

Parentheses MAY contain genuinely optional clarification.

```rust
// Acceptable: a supplementary detail.
// Integer overflow or division by zero.
Arithmetic(String),
```

When the parenthetical information affects the implementation decision, split it into a main
sentence or an additional sentence.

## 7.3 Use the right comment form

| Form | Use | Example |
|---|---|---|
| `//` | local implementation rationale or invariant | `// Short-circuit logical operators: …` |
| `///` | public item documentation | `/// Evaluate an expression against the current variable scope.` |
| `//!` | crate-level or module-level documentation | every module opens with a `//!` line |
| `// TODO(issue):` | actionable, tracked follow-up | link the tracking issue |
| `// FIXME(issue):` | known defect requiring correction | link the tracking issue |

```rust
//! The instruction-stepping interpreter over the IR.

/// Advance past control instructions, pausing at the next agent op.
pub fn step(ir: &[Instr], ctx: &mut Context) -> StepOutcome {
    // ...
}
```

Prefer `//` and `///` to multi-line `/* ... */` comments.

## 7.4 Public API documentation

Public items MUST have rustdoc when their intent, contract, or usage is not self-evident. Fallible
public functions MUST document their failure modes with `# Errors`, and any function that can
panic on a reachable path MUST document why with `# Panics`.

Public documentation SHOULD include:

1. a one-sentence summary in the present tense;
2. semantics and invariants;
3. `# Errors` for meaningful failure modes;
4. `# Panics` where callers can reach a panic;
5. `# Examples` for non-trivial APIs.

```rust
/// Lower a module into a flat instruction stream.
///
/// # Panics
/// Panics if a `Call` references a function that was not collected as a
/// definition. This is an internal invariant maintained by the parser, which
/// only emits `Call` instructions for callees present in [`Stmt::Function`].
pub fn lower(module: &Module) -> Vec<Instr> {
    // ...
}

/// Create or reset an instance: clear `dir`, write the workflow source, and
/// initialise a fresh context.
///
/// # Errors
/// Returns [`InstanceError`] on filesystem failure; the fresh context is also
/// serialised via [`save_context`].
pub fn start_instance(dir: &Path, workflow_src: &str) -> Result<(), InstanceError> {
    // ...
}
```

Do not restate information already visible in the signature.

```rust
// Incorrect
/// Takes `dir: &Path` and returns `Result<Context, InstanceError>`.
pub fn load_context(dir: &Path) -> Result<Context, InstanceError> { /* ... */ }

// Correct
/// Load the instance's execution context from `context.json`.
pub fn load_context(dir: &Path) -> Result<Context, InstanceError> { /* ... */ }
```

## 7.5 TODO and FIXME comments

Every `TODO` or `FIXME` MUST state the required action and reference a tracking item when one
exists.

```rust
// Incorrect
// TODO: improve rendering.

// Correct
// TODO(#142): cache parsed templates instead of re-parsing on every render.
```
