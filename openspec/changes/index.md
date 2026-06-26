# OpenSpec Changes Index

In-progress and completed OpenSpec changes. Each subfolder holds one change's
`proposal.md`, `design.md`, `tasks.md`, and `specs/` delta. Descend into the
one matching the work in question.

| Change | Description |
|--------|-------------|
| [enforce-return-type-on-set](./enforce-return-type-on-set/proposal.md) | Enforce the callee's declared `return` type at `set` time for value ops, so a mistyped `steer instance set` (e.g. a JSON object for a `bool`) is rejected instead of silently producing a false truthy verdict. |
