# Build and Lint Policy

## Purpose

Pins the workspace-wide build and lint configuration that gates every
contribution to steer as a contract. The pinned lint levels, edition, release
profile, and formatting settings define the quality bar all crates must meet;
relaxing them silently is a regression.

## Requirements

### Requirement: The workspace targets Rust 2021 with resolver 2 and three members

The workspace SHALL target the Rust 2021 edition with resolver "2", and SHALL
contain exactly three member crates: `steer-syntax`, `steer-core`, and
`steer-cli`. The workspace package version, edition, license, repository, and
description MUST be declared once in `[workspace.package]` and inherited by each
crate. Evidenced by `[workspace]`, `[workspace.package]` in `Cargo.toml`.

#### Scenario: edition and resolver are pinned

- **WHEN** the workspace is built
- **THEN** the edition is 2021 and the resolver is "2".

#### Scenario: the three members are present

- **WHEN** the workspace members are enumerated
- **THEN** they are exactly `steer-syntax`, `steer-core`, and `steer-cli`.

### Requirement: `unsafe_code` is forbidden and `unused_must_use` is denied workspace-wide

The workspace SHALL forbid `unsafe_code` and SHALL deny `unused_must_use` across
all crates via `[workspace.lints.rust]`. No crate SHALL relax these levels.
Evidenced by `[workspace.lints.rust]` in `Cargo.toml`.

#### Scenario: unsafe code is forbidden

- **WHEN** any crate contains an `unsafe` block
- **THEN** the build fails because `unsafe_code` is set to "forbid".

#### Scenario: ignored `must_use` results are denied

- **WHEN** any crate ignores a `must_use` result
- **THEN** the build fails because `unused_must_use` is set to "deny".

### Requirement: clippy's default set is the hard gate via deny-warnings

clippy's default set (`clippy::all`) SHALL be enabled workspace-wide at warn
level with priority -1, and the gate `cargo clippy --workspace --all-targets
--all-features -- -D warnings` SHALL turn those warnings into errors. No crate
SHALL pass with an outstanding default-set clippy warning. Evidenced by
`[workspace.lints.clippy] all = { level = "warn", priority = -1 }` in
`Cargo.toml` and the "Automated Checks (Rust)" section of `CLAUDE.md`.

#### Scenario: the clippy gate denies warnings

- **WHEN** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  is run
- **THEN** any default-set clippy warning causes a nonzero exit.

#### Scenario: default-set warnings are surfaced

- **WHEN** any crate introduces a `clippy::all` warning
- **THEN** it is reported and fails the gate.

### Requirement: `dbg_macro` warns; `todo` and `unimplemented` are denied

The workspace SHALL emit a warning for the dbg macro lint and SHALL deny the
todo and unimplemented macros. Specifically, the dbg_macro clippy lint MUST be
set to warn, and the todo and unimplemented clippy lints MUST be set to deny.
Evidenced by the workspace clippy lint table in Cargo.toml.

#### Scenario: the dbg macro warns

- **WHEN** any crate uses the dbg macro
- **THEN** clippy emits a dbg_macro warning.

#### Scenario: todo and unimplemented are denied

- **WHEN** any crate uses the todo macro or the unimplemented macro
- **THEN** the build fails because both lints are set to "deny".

### Requirement: Pedantic, unwrap, docs, and print lints are intentionally not enabled

The workspace SHALL intentionally leave several stricter lint groups
disabled. The pedantic lint group, the unwrap-used lint, the missing-docs lint,
and the print-family lints SHALL NOT be enabled workspace-wide, because the CLI
legitimately writes to stdout and the project intentionally allows unwrap in
non-test paths. Per-crate library allow-blocks SHALL document deliberate
exceptions should stricter groups be added. Evidenced by the comment block above
the workspace lints table and the per-crate allow-blocks described in CLAUDE.md.

#### Scenario: pedantic is not enabled workspace-wide

- **WHEN** the workspace lint table is inspected
- **THEN** the pedantic group is absent and the unwrap-used, missing-docs, and
  print-family lints are not set to deny or warn.

#### Scenario: deliberate per-crate exceptions are documented

- **WHEN** a stricter group is considered for a crate
- **THEN** the crate's library allow-block records the deliberate exception.

### Requirement: Each crate opts into the workspace lints

Each crate SHALL opt into the workspace lint table via `[lints] workspace = true`
in its `Cargo.toml`. No crate SHALL define a conflicting local lint table that
overrides the workspace policy. Evidenced by `[lints] workspace = true` in
`crates/steer-cli/Cargo.toml` (and the sibling crates).

#### Scenario: a crate inherits the workspace lints

- **WHEN** any member crate's `Cargo.toml` is inspected
- **THEN** it contains `[lints] workspace = true`.

### Requirement: The release profile enables LTO

The release profile SHALL enable link-time optimization via
`[profile.release] lto = true`. Evidenced by `[profile.release]` in `Cargo.toml`.

#### Scenario: release builds use LTO

- **WHEN** the workspace is built with `cargo build --release`
- **THEN** link-time optimization is enabled for release artifacts.

### Requirement: rustfmt settings are pinned

rustfmt SHALL be configured with edition 2021, `max_width = 100`,
`newline_style = "Unix"`, `use_field_init_shorthand = true`, and
`use_try_shorthand = true`. Evidenced by `rustfmt.toml`.

#### Scenario: formatting settings are applied

- **WHEN** `cargo fmt` formats the workspace
- **THEN** it applies a max width of 100, Unix newlines, field-init shorthand,
  and try shorthand, targeting edition 2021.

### Requirement: The mandatory pre-submit triple must pass before submission

Before submitting a change, the contributor SHALL run and pass all three of:
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, and `cargo test --workspace --all-features`. No
change SHALL be submitted with a failing member of this triple. Evidenced by the
"Automated Checks (Rust)" section of `CLAUDE.md`.

#### Scenario: formatting is checked

- **WHEN** `cargo fmt --all -- --check` is run before submission
- **THEN** it exits 0 (no formatting diff).

#### Scenario: clippy is checked

- **WHEN** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  is run before submission
- **THEN** it exits 0.

#### Scenario: tests are run

- **WHEN** `cargo test --workspace --all-features` is run before submission
- **THEN** the full test suite passes.

### Requirement: External dependencies are pinned workspace-wide

External dependencies SHALL be declared once in `[workspace.dependencies]` and
inherited by crates: `clap` v4 with the `derive` feature, `thiserror` v1, `serde`
v1 with the `derive` feature, and `serde_json` v1. The internal crates
`steer-syntax` and `steer-core` SHALL be declared as path dependencies pinned to
version 0.1.0. Evidenced by `[workspace.dependencies]` in `Cargo.toml`.

#### Scenario: external deps are version-pinned

- **WHEN** the workspace dependency table is inspected
- **THEN** clap is "4", thiserror is "1", serde is "1", and serde_json is "1",
  each with their documented features.

#### Scenario: internal crates are path-pinned

- **WHEN** the workspace dependency table is inspected
- **THEN** `steer-syntax` and `steer-core` are declared as path dependencies with
  version "0.1.0".
