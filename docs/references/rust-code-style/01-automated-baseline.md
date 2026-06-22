# 1. Automated Baseline

> Part of the **Rust Code Style**. See [`index.md`](./index.md) for the scope and the meaning of
> **MUST / SHOULD / MAY**.

Formatting and linting are automated checks, not review preferences.

Before submitting a change, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

During local development, use:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
```

The lint gate is clippy's **default** set (`clippy::all`), typically wired in through
`[workspace.lints]` plus each crate's `[lints] workspace = true`. Pedantic, `unwrap_used`, and
`missing_docs` are intentionally not enabled by default.

## Rules

- Code **MUST** be formatted by `rustfmt`; do not manually align spaces or reformat generated
  output. Keep the repo's `rustfmt.toml` on the default `use_small_heuristics`; raising it to `Max`
  collapses the preferred vertical layout.
- New warnings from `cargo clippy` **MUST NOT** be introduced without an explicit, local
  `#[allow(...)]` and a reason.
- Do not enable Clippy's entire `restriction` group globally. Select individual restrictions only
  after the team agrees that they fit the codebase.
- Avoid `#[allow(clippy::...)]` at crate or module scope. Prefer the narrowest item or expression
  scope possible. A crate-level `#![allow(...)]` block is acceptable as a deliberate, documented
  exception — but every entry MUST carry a reason.

```rust
// Incorrect: the allow is unexplained, so it hides every future occurrence silently.
#![allow(clippy::module_name_repetitions)]

// Correct: each allow names the reason it is needed.
#![allow(
    clippy::cast_possible_truncation, // an index type is a fixed-width integer by design
    clippy::cast_precision_loss,      // i64 -> f64 in numeric evaluation is intended
    clippy::too_many_lines,           // the main dispatch loop is one coherent unit
    clippy::module_name_repetitions,  // re-exports keep call-site names explicit
    clippy::wildcard_imports,         // `use crate::ast::*` is intentional ergonomics
    clippy::implicit_hasher,          // internal maps use the default hasher
    clippy::many_single_char_names,   // short loop counters and match binders (i, n, s)
    clippy::must_use_candidate,       // `#[must_use]` is added only where dropping is a bug
    clippy::single_char_pattern,      // single-char string patterns read fine in rendering
    clippy::enum_glob_use,            // tight operator-dispatch tables use `use Op::*`
    clippy::single_match_else,        // a two-arm `match` is often clearer than `if/else`
    clippy::if_not_else,              // condition polarity follows the surrounding logic
    clippy::map_unwrap_or             // `.map(f).unwrap_or(a)` is preferred here for ordering
)]
```
