# 10. Formatting Details

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**. Most of this is enforced by `rustfmt`; the points below are the
> human-judgement parts.

- Use four-space indentation. Let `rustfmt` make final line-breaking decisions.
- Use trailing commas in multi-line lists, struct literals, match arms, and function calls.
- Keep expressions readable rather than optimizing for minimum line count. Keep `rustfmt.toml` on
  the default `use_small_heuristics`, so borderline collections expand to vertical layout.
- Use one attribute per line, except combine multiple derives in one `#[derive(...)]`.
- Place rustdoc (`///`) before attributes.

```rust
/// One rendered instruction in the dry-run output.
#[derive(Debug, Clone, PartialEq)]
pub struct SimStep {
    /// The callee name, e.g. `task`, `ask`, or `command`.
    pub callee: String,
    /// The rendered instruction text.
    pub instruction: String,
}
```

Prefer a trailing comma and vertical layout when a list becomes hard to scan. Lay out a struct
with several fields vertically.

```rust
// Incorrect: compact but difficult to extend and review.
let frame = Frame { return_pc, into, saved_vars };

// Correct
let frame = Frame {
    return_pc,
    into,
    saved_vars,
};
```

Do not align unrelated end-of-line comments manually.

```rust
// Incorrect
let pc = ctx.pc;            // Read PC.
let status = ctx.status;    // Read status.

// Correct
let pc = ctx.pc;
let status = ctx.status;
```
