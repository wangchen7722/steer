# Rust Code Style

> **Scope:** This document defines source-level Rust conventions: formatting and lint gates, file
> and module organization, naming, types and data modeling, error handling, control flow,
> comments and rustdoc, async and concurrency, tests, and formatting details. It is intentionally
> project-neutral so it can be reused across Rust codebases.
>
> The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** have the following
> meaning:
>
> - **MUST / MUST NOT**: required for consistency, correctness, or maintainability.
> - **SHOULD / SHOULD NOT**: the default rule; deviations need a clear local reason.
> - **MAY**: permitted when it improves clarity.

The guide is split into focused pages. Read the page relevant to your change rather than the whole
guide.

| # | Topic | What it covers |
|---|-------|----------------|
| 1 | [Automated Baseline](./01-automated-baseline.md) | `cargo fmt` / `cargo clippy` / `cargo test` gates; rules for `#[allow(...)]`. |
| 2 | [File and Module Organization](./02-file-and-module-organization.md) | File section order, import grouping, single-responsibility module boundaries. |
| 3 | [Naming](./03-naming.md) | Casing, domain-meaningful names, getters, conversion prefixes, constructors. |
| 4 | [Types, APIs, and Data Modeling](./04-types-apis-data-modeling.md) | Newtypes, enums for state, small traits, `Option`/`Result`. |
| 5 | [Error Handling](./05-error-handling.md) | Preserving context at boundaries, no panics, clear error messages. |
| 6 | [Control Flow](./06-control-flow.md) | Early returns, exhaustive `match`, readable boolean expressions. |
| 7 | [Comments and Rustdoc](./07-comments-and-rustdoc.md) | Why-not-what, comment forms, public API docs, TODO/FIXME. |
| 8 | [Async and Concurrency](./08-async-and-concurrency.md) | Locks across `.await`, cancellation/ownership, shared mutable state. |
| 9 | [Tests](./09-tests.md) | Behavior over implementation, semantic assertions, test boundaries. |
| 10 | [Formatting Details](./10-formatting-details.md) | Indentation, trailing commas, attribute layout, comment alignment. |
