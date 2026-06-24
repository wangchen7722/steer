# workflow-marketplace-install Implementation Plan

> **For agentic workers:** The `openspec-apply` workflow executes this plan
> task-by-task via `superpowers:subagent-driven-development`. Steps use checkbox
> (`- [ ]`) syntax for tracking. Do not skip TDD: RED → GREEN → REFACTOR, one
> commit per task.

**Goal:** Ship `steer workflow install`, an interactive package-manager command that clones a marketplace repo, lets the user multi-select workflows in a ratatui TUI, and copies each selected workflow and its `@template` template sets into `.steer/`.

**Architecture:** All world-touching code (git, terminal, filesystem writes) lives in a new crate `crates/steer-marketplace`, so `steer-core`/`steer-syntax` stay pure. `steer-cli` gains a thin `WorkflowAction::Install` variant that delegates to `steer_marketplace::install`. `steer-core` gains one additive helper, `workflow_template(&Module) -> Option<String>`, mirroring the existing `workflow_description`. Modules: `source` (URL resolution, pure), `git` (clone + RAII temp guard), `catalog` (scan), `conflict` (detect + resolve + backup), `tui` (selection UI), `install` (orchestrator).

**Tech Stack:** Rust 2021; `clap` (existing); new deps `ratatui` 0.28 (+`crossterm` backend), `crossterm` 0.28, `toml` 0.8; `tempfile` 3 as a dev-dependency for tests; system `git` binary for cloning.

**Files:**

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add workspace deps (ratatui, crossterm, toml) + `steer-marketplace` member/path |
| Modify | `crates/steer-core/src/storage.rs` | Add `workflow_template` helper |
| Modify | `crates/steer-core/src/lib.rs` | Re-export `workflow_template` |
| Modify | `crates/steer-cli/Cargo.toml` | Depend on `steer-marketplace` |
| Modify | `crates/steer-cli/src/main.rs` | `WorkflowAction::Install` + `run_install` |
| Create | `crates/steer-marketplace/Cargo.toml` | Crate manifest |
| Create | `crates/steer-marketplace/src/lib.rs` | Data model + module wiring + `install` re-export |
| Create | `crates/steer-marketplace/src/source.rs` | URL/registry/env resolution |
| Create | `crates/steer-marketplace/src/git.rs` | `git` clone + `TempGuard` |
| Create | `crates/steer-marketplace/src/catalog.rs` | Scan cloned tree into `CatalogEntry` |
| Create | `crates/steer-marketplace/src/conflict.rs` | Detect/resolve conflicts, backup naming, apply copies |
| Create | `crates/steer-marketplace/src/tui.rs` | ratatui multi-select + key handling |
| Create | `crates/steer-marketplace/src/install.rs` | Orchestrator + summary + exit codes |
| Modify | `README.md` | Document the command, flags, marketplace layout, trust note |
| Create | `docs/specs/workflow-install.md` | Behavior spec |
| Modify | `docs/specs/index.md` | List the new spec |

## Global Constraints

- Lint gate is `cargo clippy --workspace --all-targets --all-features -- -D warnings`; workspace denies `unsafe_code`, `unused_must_use`, `todo!`, `unimplemented!`; warns on `clippy::all` and `dbg_macro`. `unwrap_used`/`missing_docs`/pedantic are NOT enabled, so `.unwrap()`/`.expect()` are allowed in tests.
- Because `unused_must_use = deny`, every `Result`/must-use return (e.g. `fs::create_dir_all`, `fs::rename`, `fs::copy`) must be consumed (`?`, `let _ =`, or `.ok()`); in `Drop` use `let _ =`.
- Formatting gate: `cargo fmt --all -- --check`. Match existing 4-space style.
- Commit messages MUST NOT include `Co-Authored-By`. Conventional-commit style.
- Work happens on branch `workflow-marketplace-install-dev`; `master` is integration-only.
- File/code artifacts in English.

---

### Task 1: Workspace wiring, crate scaffold, `workflow_template`, data model

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/steer-core/src/storage.rs` (after `workflow_description`, ~line 110)
- Modify: `crates/steer-core/src/lib.rs` (re-export line ~44)
- Create: `crates/steer-marketplace/Cargo.toml`
- Create: `crates/steer-marketplace/src/lib.rs`
- Test: `crates/steer-core/src/storage.rs` (unit tests), `crates/steer-marketplace/src/lib.rs` (compile)

**Interfaces:**
- Produces: `steer_core::workflow_template(&steer_syntax::Module) -> Option<String>`; in the new crate: `CatalogEntry`, `ConflictPolicy`, `CopyOp`, `InstallArgs`, and module declarations.

- [ ] **Step 1: Write the failing test (steer-core)**

Append to the `#[cfg(test)] mod tests` block in `crates/steer-core/src/storage.rs`:

```rust
    fn workflow_template_extracts_directive() {
        let m = steer_syntax::parse("@template = \"openspec-superpowers\"\ntask(\"x\")\n")
            .expect("parses");
        assert_eq!(workflow_template(&m).as_deref(), Some("openspec-superpowers"));
    }

    #[test]
    fn workflow_template_absent_is_none() {
        let m = steer_syntax::parse("@description = \"d\"\ntask(\"x\")\n").expect("parses");
        assert_eq!(workflow_template(&m), None);
    }

    #[test]
    fn workflow_template_empty_is_none() {
        let m = steer_syntax::parse("@template = \"\"\ntask(\"x\")\n").expect("parses");
        assert_eq!(workflow_template(&m), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p steer-core workflow_template`
Expected: FAIL — `cannot find function workflow_template`.

- [ ] **Step 3: Write minimal implementation**

In `crates/steer-core/src/storage.rs`, immediately after the `workflow_description` function:

```rust
/// Extract the top-level `@template = "..."` directive from a parsed workflow
/// module. Mirrors [`workflow_description`]: the literal is rendered to text and
/// an empty result is treated as absent (`None`). Used by `steer workflow
/// install` to resolve which template set(s) a marketplace workflow brings.
pub fn workflow_template(module: &steer_syntax::Module) -> Option<String> {
    for s in &module.body {
        if let steer_syntax::ast::Stmt::Meta { key, value } = &s.value {
            if key == "template" {
                let rendered = eval_literal(value).render();
                return if rendered.is_empty() {
                    None
                } else {
                    Some(rendered)
                };
            }
        }
    }
    None
}
```

In `crates/steer-core/src/lib.rs`, extend the `pub use storage::{ ... }` block to include `workflow_template`:

```rust
pub use storage::{
    load_context, load_ir, save_context, start_instance, workflow_description, workflow_template,
    InstanceError,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p steer-core workflow_template`
Expected: PASS (3 tests).

- [ ] **Step 5: Add the new crate scaffold**

Append to `Cargo.toml` `[workspace]`:

```toml
members = ["crates/steer-syntax", "crates/steer-core", "crates/steer-cli", "crates/steer-marketplace"]
```

Add to `[workspace.dependencies]`:

```toml
ratatui = { version = "0.28", features = ["crossterm"] }
crossterm = "0.28"
toml = "0.8"
tempfile = "3"
steer-marketplace = { path = "crates/steer-marketplace", version = "0.1.0" }
```

Create `crates/steer-marketplace/Cargo.toml`:

```toml
[package]
name = "steer-marketplace"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "steer workflow marketplace: the install/package-manager side of steer."

[dependencies]
steer-syntax.workspace = true
steer-core.workspace = true
serde.workspace = true
ratatui.workspace = true
crossterm.workspace = true
toml.workspace = true

[dev-dependencies]
tempfile.workspace = true

[lints]
workspace = true
```

Create `crates/steer-marketplace/src/lib.rs`:

```rust
//! The package-manager side of steer: `steer workflow install`.
//!
//! This crate is the only part of steer that touches the outside world (git,
//! the terminal, the filesystem-write path). The instance runtime in
//! `steer-core` stays pure; install runs outside any instance.

use std::path::PathBuf;

/// One workflow offered by a marketplace, scanned from the cloned tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// Workflow name = file stem of its `.steer` file.
    pub name: String,
    /// The workflow's `@description`, if any.
    pub description: Option<String>,
    /// Template set names read from the workflow's `@template` directive.
    pub template_sets: Vec<String>,
    /// Path to the workflow's `.steer` file inside the cloned tree.
    pub workflow_path: PathBuf,
    /// Path to the cloned catalog base (`<root>` or `<root>/.steer`).
    pub base: PathBuf,
}

/// How to resolve a target file that already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// Keep the existing file; do not write the marketplace copy.
    Skip,
    /// Replace the existing file with the marketplace copy.
    Overwrite,
    /// Rename the existing file to a `.bak[.N]` backup, then write the copy.
    Backup,
    /// Prompt per conflict (interactive only); resolved before any copy.
    Ask,
}

/// A single file copy planned for the install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyOp {
    pub src: PathBuf,
    pub dst: PathBuf,
}

/// Parsed CLI arguments, passed in from `steer-cli`.
#[derive(Debug, Clone, Default)]
pub struct InstallArgs {
    pub marketplace: Option<String>,
    pub workflows: Option<String>,
    pub all: bool,
    pub force: bool,
    pub skip: bool,
    pub backup: bool,
    pub dry_run: bool,
    pub git_ref: Option<String>,
}

pub mod catalog;
pub mod conflict;
pub mod git;
pub mod install;
pub mod source;
pub mod tui;

pub use install::install;
```

- [ ] **Step 6: Build the workspace**

Run: `cargo build -p steer-marketplace 2>&1 | head -40`
Expected: errors about missing modules `catalog`/`conflict`/`git`/`install`/`source`/`tui` — that is expected; create empty module stubs so it compiles. Create each file with a doc comment only, e.g. `crates/steer-marketplace/src/source.rs`:

```rust
//! Marketplace source resolution (flag / registry / env). Implemented in later tasks.
```

Repeat for `git.rs`, `catalog.rs`, `conflict.rs`, `tui.rs`. For `install.rs`:

```rust
//! Install orchestrator. Implemented in later tasks.
use std::process::ExitCode;
use crate::InstallArgs;

pub fn install(_args: InstallArgs) -> ExitCode {
    ExitCode::FAILURE
}
```

Run: `cargo build --workspace`
Expected: PASS (workspace compiles).

- [ ] **Step 7: Commit**

```bash
git checkout -b workflow-marketplace-install-dev
git add Cargo.toml crates/steer-core/src/storage.rs crates/steer-core/src/lib.rs crates/steer-marketplace
git commit -m "feat(marketplace): scaffold steer-marketplace crate and workflow_template helper"
```

---

### Task 2: `looks_like_url` (source.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/source.rs`
- Test: same file (`#[cfg(test)]`)

**Interfaces:**
- Produces: `pub fn looks_like_url(value: &str) -> bool`.

- [ ] **Step 1: Write the failing test**

Replace the placeholder content of `crates/steer-marketplace/src/source.rs` with:

```rust
//! Marketplace source resolution: `--marketplace` flag, named registry,
//! and `STEER_MARKETPLACE_URL` env var.

/// True when `value` should be treated as a direct URL rather than a name.
pub fn looks_like_url(value: &str) -> bool {
    value.contains("://") || value.ends_with(".git")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_is_a_url() {
        assert!(looks_like_url("https://github.com/foo/bar"));
    }

    #[test]
    fn ssh_dot_git_is_a_url() {
        assert!(looks_like_url("git@github.com:foo/bar.git"));
    }

    #[test]
    fn plain_name_is_not_a_url() {
        assert!(!looks_like_url("official"));
        assert!(!looks_like_url("community-v2"));
    }
}
```

- [ ] **Step 2: Run test to verify it passes (GREEN already minimal)**

Run: `cargo test -p steer-marketplace source::tests`
Expected: PASS (the implementation is one line and already correct).

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/source.rs
git commit -m "feat(marketplace): detect direct marketplace URLs"
```

---

### Task 3: Registry parsing (source.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/source.rs`

**Interfaces:**
- Produces: `struct Registry`, `struct RegistryEntry`, `pub fn load_registry(&Path) -> Result<Option<Registry>, String>`.

- [ ] **Step 1: Write the failing test**

Add above the `#[cfg(test)]` line:

```rust
use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// Named-registry file: `.steer/marketplaces.toml` mapping names to URLs.
#[derive(Debug, Deserialize, Default)]
pub struct Registry {
    #[serde(default)]
    pub marketplaces: BTreeMap<String, RegistryEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryEntry {
    pub url: String,
}

/// Load and parse a registry file. `Ok(None)` when the file is absent (not an
/// error); `Err` when it exists but is malformed TOML.
pub fn load_registry(path: &Path) -> Result<Option<Registry>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read registry {}: {e}", path.display())),
    };
    toml::from_str::<Registry>(&text)
        .map(Some)
        .map_err(|e| format!("malformed registry {}: {e}", path.display()))
}
```

Add tests inside the existing `mod tests`:

```rust
    #[test]
    fn load_registry_absent_is_none() {
        let p = std::env::temp_dir().join("sm-no-such-registry.toml");
        let _ = std::fs::remove_file(&p);
        assert!(matches!(load_registry(&p), Ok(None)));
    }

    #[test]
    fn load_registry_parses_entries() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("marketplaces.toml");
        std::fs::write(
            &p,
            "[marketplaces.official]\nurl = \"https://example.com/a\"\n[marketplaces.community]\nurl = \"https://example.com/b\"\n",
        )
        .unwrap();
        let reg = load_registry(&p).unwrap().unwrap();
        assert_eq!(reg.marketplaces["official"].url, "https://example.com/a");
        assert_eq!(reg.marketplaces["community"].url, "https://example.com/b");
    }

    #[test]
    fn load_registry_malformed_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.toml");
        std::fs::write(&p, "this is = = not toml").unwrap();
        let err = load_registry(&p).unwrap_err();
        assert!(err.contains("malformed registry"), "{err}");
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace source::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/source.rs
git commit -m "feat(marketplace): parse named-marketplace registry (TOML)"
```

---

### Task 4: Resolution precedence (source.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/source.rs`

**Interfaces:**
- Produces: `struct ResolvedSource`, `pub fn resolve(Option<&str>, &[PathBuf], Option<&str>) -> Result<ResolvedSource, String>`, `pub fn user_registry_path() -> Option<PathBuf>`.
- Consumes: `looks_like_url`, `load_registry`.

- [ ] **Step 1: Write the failing test**

Add to `source.rs` (above `#[cfg(test)]`):

```rust
use std::path::PathBuf;

/// A marketplace URL plus a human label for messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    pub url: String,
    pub label: String,
}

/// Path to the user-global registry `~/.steer/marketplaces.toml`, if a home
/// directory can be determined from `HOME` / `USERPROFILE`.
pub fn user_registry_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".steer").join("marketplaces.toml"))
}

/// Resolve the marketplace URL by precedence: `--marketplace <url>` >
/// `--marketplace <name>` (looked up across `registries` in order) >
/// `STEER_MARKETPLACE_URL` > error. Each registry is loaded with
/// [`load_registry`]; a missing file is skipped, a malformed one is fatal.
pub fn resolve(
    marketplace: Option<&str>,
    registries: &[PathBuf],
    env_url: Option<&str>,
) -> Result<ResolvedSource, String> {
    if let Some(value) = marketplace {
        if looks_like_url(value) {
            return Ok(ResolvedSource {
                url: value.to_string(),
                label: format!("--marketplace {value}"),
            });
        }
        for reg_path in registries {
            if let Some(reg) = load_registry(reg_path)? {
                if let Some(entry) = reg.marketplaces.get(value) {
                    return Ok(ResolvedSource {
                        url: entry.url.clone(),
                        label: format!("--marketplace {value}"),
                    });
                }
            }
        }
        return Err(format!(
            "unknown marketplace `{value}`: not found in any registry. Register it under [marketplaces.{value}] in .steer/marketplaces.toml or pass --marketplace <url>."
        ));
    }
    if let Some(url) = env_url.filter(|s| !s.is_empty()) {
        return Ok(ResolvedSource {
            url: url.to_string(),
            label: "STEER_MARKETPLACE_URL".to_string(),
        });
    }
    Err(
        "no marketplace source configured. Set STEER_MARKETPLACE_URL, pass --marketplace <url>, or register a named marketplace in .steer/marketplaces.toml."
            .to_string(),
    )
}
```

Add tests:

```rust
    #[test]
    fn resolve_direct_url_wins() {
        let r = resolve(Some("https://x.example/y"), &[], Some("https://env.example/z"));
        assert_eq!(r.unwrap().url, "https://x.example/y");
    }

    #[test]
    fn resolve_named_from_first_registry() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("marketplaces.toml");
        std::fs::write(&p, "[marketplaces.official]\nurl = \"https://r.example\"\n").unwrap();
        let r = resolve(Some("official"), &[p], None).unwrap();
        assert_eq!(r.url, "https://r.example");
    }

    #[test]
    fn resolve_named_first_registry_wins_over_second() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let pa = a.path().join("m.toml");
        let pb = b.path().join("m.toml");
        std::fs::write(&pa, "[marketplaces.x]\nurl = \"https://a\"\n").unwrap();
        std::fs::write(&pb, "[marketplaces.x]\nurl = \"https://b\"\n").unwrap();
        let r = resolve(Some("x"), &[pa, pb], None).unwrap();
        assert_eq!(r.url, "https://a");
    }

    #[test]
    fn resolve_env_when_no_flag() {
        let r = resolve(None, &[], Some("https://env.example")).unwrap();
        assert_eq!(r.url, "https://env.example");
    }

    #[test]
    fn resolve_empty_env_is_unset() {
        let err = resolve(None, &[], Some("")).unwrap_err();
        assert!(err.contains("no marketplace source"));
    }

    #[test]
    fn resolve_unknown_name_is_error() {
        let err = resolve(Some("ghost"), &[], None).unwrap_err();
        assert!(err.contains("unknown marketplace `ghost`"));
    }

    #[test]
    fn resolve_malformed_registry_is_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.toml");
        std::fs::write(&p, "bad = = toml").unwrap();
        let err = resolve(Some("official"), &[p], None).unwrap_err();
        assert!(err.contains("malformed registry"));
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace source::tests`
Expected: PASS (all source tests).

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/source.rs
git commit -m "feat(marketplace): resolve marketplace URL by flag/registry/env precedence"
```

---

### Task 5: `TempGuard` (git.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/git.rs`

**Interfaces:**
- Produces: `pub struct TempGuard`, `TempGuard::new() -> io::Result<Self>`, `TempGuard::path() -> &Path`.

- [ ] **Step 1: Write the failing test**

Replace `git.rs` content with:

```rust
//! Shallow-clone the marketplace via system `git` into a temp dir guarded by RAII.

use std::path::{Path, PathBuf};

/// RAII guard owning a temp directory; removed on drop unless `keep()`.
pub struct TempGuard {
    dir: PathBuf,
    keep: bool,
}

impl TempGuard {
    /// Create a fresh temp dir under the system temp area, named by PID.
    pub fn new() -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!("steer-marketplace-{}", std::process::id()));
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir, keep: false })
    }

    /// Path of the owned temp directory.
    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Keep the directory on drop (for debugging failed clones).
    pub fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_guard_creates_and_removes() {
        let owned;
        let path;
        {
            let g = TempGuard::new().unwrap();
            path = g.path().to_path_buf();
            assert!(path.is_dir());
            owned = g;
        }
        drop(owned);
        assert!(!path.exists());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace git::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/git.rs
git commit -m "feat(marketplace): RAII temp-dir guard for clones"
```

---

### Task 6: git presence + clone (git.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/git.rs`

**Interfaces:**
- Produces: `pub fn git_present() -> bool`, `pub fn clone(&str, Option<&str>, &Path) -> Result<(), String>`.

- [ ] **Step 1: Write the failing test**

Append to `git.rs` (above `#[cfg(test)]`):

```rust
use std::process::{Command, Stdio};

/// True iff the `git` binary is available on `PATH`.
pub fn git_present() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Shallow-clone `url` into `dest`. `git_ref`, if given, is passed via
/// `--branch` (accepts a branch or tag; arbitrary commit SHAs may not resolve
/// under `--depth 1`). Returns an error message on failure.
pub fn clone(url: &str, git_ref: Option<&str>, dest: &Path) -> Result<(), String> {
    if !git_present() {
        return Err("git is required but was not found on PATH".to_string());
    }
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(r) = git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(url).arg(dest);
    let status = cmd
        .status()
        .map_err(|e| format!("failed to spawn git: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "git clone failed for {url} (exit {:?})",
            status.code()
        ))
    }
}
```

Add the test (uses a real local repo created on the fly; skipped if git is absent):

```rust
    #[test]
    fn clone_local_fixture_repo() {
        if !git_present() {
            eprintln!("skipping clone test: git not present");
            return;
        }
        // Create a bare origin with one commit.
        let origin = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        for (args, dir) in [
            (["init", "-q"].as_slice(), origin.path()),
            (["init", "-q"].as_slice(), work.path()),
        ] {
            let _ = Command::new("git").current_dir(dir).args(args).status();
        }
        // Make the origin a non-bare repo with a commit, then clone it.
        let _ = Command::new("git")
            .current_dir(origin.path())
            .args(["config", "user.email", "t@t"])
            .status();
        let _ = Command::new("git")
            .current_dir(origin.path())
            .args(["config", "user.name", "t"])
            .status();
        std::fs::write(origin.path().join("workflows/alpha.steer"), "@description = \"a\"\ntask(\"x\")\n")
            .unwrap(); // Note: parent dir may not exist yet; create it.
        std::fs::create_dir_all(origin.path().join("workflows")).unwrap();
        std::fs::write(origin.path().join("workflows/alpha.steer"), "@description = \"a\"\ntask(\"x\")\n")
            .unwrap();
        let _ = Command::new("git").current_dir(origin.path()).args(["add", "."]).status();
        let _ = Command::new("git").current_dir(origin.path()).args(["commit", "-qm", "c"]).status();

        let dest = tempfile::tempdir().unwrap();
        let url = origin.path().to_str().unwrap();
        clone(url, None, dest.path()).expect("clone succeeds");
        assert!(dest.path().join("workflows/alpha.steer").is_file());
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace git::tests`
Expected: PASS (the redundant first `std::fs::write` will fail silently via `.unwrap()` — remove the duplicate line before committing; keep only the version after `create_dir_all`). Fix: delete the first `std::fs::write(...).unwrap();` block so only the post-`create_dir_all` write remains.

- [ ] **Step 3: Refactor (remove the duplicate write)**

Delete these two lines from the test:

```rust
        std::fs::write(origin.path().join("workflows/alpha.steer"), "@description = \"a\"\ntask(\"x\")\n")
            .unwrap(); // Note: parent dir may not exist yet; create it.
```

Run again: `cargo test -p steer-marketplace git::tests` → PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/steer-marketplace/src/git.rs
git commit -m "feat(marketplace): shallow git clone with presence pre-flight"
```

---

### Task 7: catalog base resolution (catalog.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/catalog.rs`

**Interfaces:**
- Produces: `pub fn catalog_base(&Path) -> PathBuf`.

- [ ] **Step 1: Write the failing test**

Replace `catalog.rs` content with:

```rust
//! Scan the cloned marketplace tree into catalog entries.

use std::path::{Path, PathBuf};

/// Resolve the catalog base: `<root>/.steer` when it contains a `workflows/`
/// or `templates/` directory, else `<root>`.
pub fn catalog_base(root: &Path) -> PathBuf {
    let inner = root.join(".steer");
    if inner.join("workflows").is_dir() || inner.join("templates").is_dir() {
        inner
    } else {
        root.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_is_root_when_workflows_at_root() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("workflows")).unwrap();
        assert_eq!(catalog_base(d.path()), d.path());
    }

    #[test]
    fn base_is_steer_dir_when_nested() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join(".steer").join("workflows")).unwrap();
        assert_eq!(catalog_base(d.path()), d.path().join(".steer"));
    }

    #[test]
    fn base_is_root_when_no_workflows_anywhere() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(catalog_base(d.path()), d.path());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace catalog::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/catalog.rs
git commit -m "feat(marketplace): resolve marketplace catalog base"
```

---

### Task 8: catalog scan (catalog.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/catalog.rs`

**Interfaces:**
- Produces: `pub fn scan(&Path) -> Vec<CatalogEntry>`.
- Consumes: `crate::CatalogEntry`, `steer_syntax::parse`, `steer_core::workflow_description`, `steer_core::workflow_template`.

- [ ] **Step 1: Write the failing test**

Append to `catalog.rs` (above `#[cfg(test)]`):

```rust
use crate::CatalogEntry;

/// Scan `<base>/workflows/*.steer` into catalog entries, sorted by name.
/// An unparseable file is listed with an `(unparseable)` description; an empty
/// catalog returns `vec![]`.
pub fn scan(base: &Path) -> Vec<CatalogEntry> {
    let mut entries = Vec::new();
    let workflows = base.join("workflows");
    if let Ok(rd) = std::fs::read_dir(&workflows) {
        for ent in rd.flatten() {
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("steer") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            entries.push(entry_for(stem.to_string(), &path, base));
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn entry_for(name: String, path: &Path, base: &Path) -> CatalogEntry {
    let src = std::fs::read_to_string(path).unwrap_or_default();
    match steer_syntax::parse(&src) {
        Ok(module) => CatalogEntry {
            name,
            description: steer_core::workflow_description(&module),
            template_sets: steer_core::workflow_template(&module)
                .map(|t| vec![t])
                .unwrap_or_default(),
            workflow_path: path.to_path_buf(),
            base: base.to_path_buf(),
        },
        Err(_) => CatalogEntry {
            name,
            description: Some("(unparseable)".to_string()),
            template_sets: vec![],
            workflow_path: path.to_path_buf(),
            base: base.to_path_buf(),
        },
    }
}
```

Add tests:

```rust
    fn write(base: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(base.join("workflows")).unwrap();
        std::fs::write(base.join("workflows").join(format!("{name}.steer")), body).unwrap();
    }

    #[test]
    fn scan_lists_workflows_with_description_and_template() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "alpha",
            "@description = \"Alpha workflow\"\n@template = \"alpha-set\"\ntask(\"x\")\n",
        );
        write(d.path(), "beta", "task(\"y\")\n");
        let entries = scan(d.path());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[0].description.as_deref(), Some("Alpha workflow"));
        assert_eq!(entries[0].template_sets, vec!["alpha-set".to_string()]);
        assert_eq!(entries[1].name, "beta");
        assert!(entries[1].description.is_none());
        assert!(entries[1].template_sets.is_empty());
    }

    #[test]
    fn scan_empty_is_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(scan(d.path()).is_empty());
    }

    #[test]
    fn scan_marks_unparseable() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "broken", "this is = = not steer");
        let entries = scan(d.path());
        assert_eq!(entries[0].description.as_deref(), Some("(unparseable)"));
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace catalog::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/catalog.rs
git commit -m "feat(marketplace): scan marketplace catalog into entries"
```

---

### Task 9: backup-name disambiguation (conflict.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/conflict.rs`

**Interfaces:**
- Produces: `pub fn backup_name(&Path) -> PathBuf`.

- [ ] **Step 1: Write the failing test**

Replace `conflict.rs` content with:

```rust
//! Detect target conflicts and resolve them: global flags, an interactive
//! prompt (default skip), and `.bak[.N]` backups. Pure policy logic is
//! separated from terminal I/O so it is unit-testable.

use std::path::{Path, PathBuf};

use crate::{CatalogEntry, ConflictPolicy, CopyOp};

/// Compute the backup path for `path`: `path.bak`, or `path.bak.1`,
/// `path.bak.2`, … so no prior backup is clobbered.
pub fn backup_name(path: &Path) -> PathBuf {
    let first = PathBuf::from(format!("{}.bak", path.display()));
    if !first.exists() {
        return first;
    }
    let mut n = 1u32;
    loop {
        let candidate = PathBuf::from(format!("{}.bak.{}", path.display(), n));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_backup_takes_bak() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("wf.steer");
        std::fs::write(&f, "x").unwrap();
        assert_eq!(backup_name(&f), d.path().join("wf.steer.bak"));
    }

    #[test]
    fn second_backup_disambiguates() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("wf.steer");
        std::fs::write(&f, "x").unwrap();
        std::fs::write(d.path().join("wf.steer.bak"), "old").unwrap();
        assert_eq!(backup_name(&f), d.path().join("wf.steer.bak.1"));
        std::fs::write(d.path().join("wf.steer.bak.1"), "older").unwrap();
        assert_eq!(backup_name(&f), d.path().join("wf.steer.bak.2"));
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace conflict::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/conflict.rs
git commit -m "feat(marketplace): backup-name disambiguation"
```

---

### Task 10: plan_copies + detect_conflicts (conflict.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/conflict.rs`

**Interfaces:**
- Produces: `pub fn plan_copies(&[CatalogEntry], &Path) -> Vec<CopyOp>`, `pub fn detect_conflicts(&[CopyOp]) -> (Vec<CopyOp>, Vec<CopyOp>)`.

- [ ] **Step 1: Write the failing test**

Append to `conflict.rs` (above `#[cfg(test)]`):

```rust
use std::collections::BTreeSet;

/// Compute every CopyOp for `entries` against `steer_dir` (the `.steer` dir):
/// each workflow's `.steer` file, plus every file under its referenced template
/// sets. Template sets are de-duplicated across the selection. A referenced set
/// absent from the marketplace contributes no ops (the caller warns).
pub fn plan_copies(entries: &[CatalogEntry], steer_dir: &Path) -> (Vec<CopyOp>, Vec<String>) {
    let mut ops = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut missing: Vec<String> = Vec::new();
    for e in entries {
        ops.push(CopyOp {
            src: e.workflow_path.clone(),
            dst: steer_dir
                .join("workflows")
                .join(format!("{}.steer", e.name)),
        });
        for set in &e.template_sets {
            if !seen.insert(set.clone()) {
                continue;
            }
            let src_dir = e.base.join("templates").join(set);
            let dst_dir = steer_dir.join("templates").join(set);
            if src_dir.is_dir() {
                ops.extend(copy_dir_ops(&src_dir, &dst_dir));
            } else {
                missing.push(set.clone());
            }
        }
    }
    (ops, missing)
}

fn copy_dir_ops(src_dir: &Path, dst_dir: &Path) -> Vec<CopyOp> {
    let mut ops = Vec::new();
    let mut stack = vec![(src_dir.to_path_buf(), dst_dir.to_path_buf())];
    while let Some((s, d)) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&s) {
            for ent in rd.flatten() {
                let sp = ent.path();
                let dp = d.join(ent.file_name());
                if sp.is_dir() {
                    stack.push((sp, dp));
                } else {
                    ops.push(CopyOp { src: sp, dst: dp });
                }
            }
        }
    }
    ops
}

/// Split `ops` into (conflicting, clean) based on whether each `dst` exists.
pub fn detect_conflicts(ops: &[CopyOp]) -> (Vec<CopyOp>, Vec<CopyOp>) {
    let mut conf = Vec::new();
    let mut clean = Vec::new();
    for op in ops {
        if op.dst.exists() {
            conf.push(op.clone());
        } else {
            clean.push(op.clone());
        }
    }
    (conf, clean)
}
```

Add tests:

```rust
    use crate::{CatalogEntry};

    fn entry(name: &str, base: &Path, tmpl: Option<&str>) -> CatalogEntry {
        CatalogEntry {
            name: name.to_string(),
            description: None,
            template_sets: tmpl.map(|t| vec![t.to_string()]).unwrap_or_default(),
            workflow_path: base.join("workflows").join(format!("{name}.steer")),
            base: base.to_path_buf(),
        }
    }

    #[test]
    fn plan_copies_workflow_and_template_files() {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("workflows")).unwrap();
        std::fs::write(src.path().join("workflows/a.steer"), "x").unwrap();
        std::fs::create_dir_all(src.path().join("templates/t")).unwrap();
        std::fs::write(src.path().join("templates/t/one.j2.md"), "x").unwrap();

        let dst = tempfile::tempdir().unwrap();
        let (ops, missing) =
            plan_copies(&[entry("a", src.path(), Some("t"))], dst.path());
        assert!(missing.is_empty());
        assert!(ops.iter().any(|o| o.dst == dst.path().join("workflows").join("a.steer")));
        assert!(ops.iter().any(|o| o.dst == dst.path().join("templates").join("t").join("one.j2.md")));
    }

    #[test]
    fn plan_copies_dedupes_shared_template_set() {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("templates/shared")).unwrap();
        std::fs::write(src.path().join("templates/shared/x.j2.md"), "x").unwrap();
        let dst = tempfile::tempdir().unwrap();
        let e1 = entry("a", src.path(), Some("shared"));
        let e2 = entry("b", src.path(), Some("shared"));
        let (ops, _) = plan_copies(&[e1, e2], dst.path());
        // one workflow file each (2) + exactly one template file (shared deduped)
        let tmpl = ops
            .iter()
            .filter(|o| o.dst.starts_with(dst.path().join("templates")))
            .count();
        assert_eq!(tmpl, 1);
    }

    #[test]
    fn plan_copies_reports_missing_template_set() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let (_, missing) = plan_copies(&[entry("a", src.path(), Some("ghost"))], dst.path());
        assert_eq!(missing, vec!["ghost".to_string()]);
    }

    #[test]
    fn detect_conflicts_splits_existing() {
        let dst = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dst.path().join("workflows")).unwrap();
        std::fs::write(dst.path().join("workflows/a.steer"), "old").unwrap();
        let op_existing = CopyOp {
            src: PathBuf::from("/tmp/x"),
            dst: dst.path().join("workflows").join("a.steer"),
        };
        let op_new = CopyOp {
            src: PathBuf::from("/tmp/y"),
            dst: dst.path().join("workflows").join("b.steer"),
        };
        let (conf, clean) = detect_conflicts(&[op_existing, op_new]);
        assert_eq!(conf.len(), 1);
        assert_eq!(clean.len(), 1);
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace conflict::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/conflict.rs
git commit -m "feat(marketplace): plan copy ops and detect conflicts"
```

---

### Task 11: policy flags + answer interpretation (conflict.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/conflict.rs`

**Interfaces:**
- Produces: `pub enum Answer`, `pub fn interpret(Answer) -> (ConflictPolicy, bool)`, `pub fn global_policy(InstallArgs-like flags) -> Option<ConflictPolicy>`, `pub fn resolve_policy(Option<ConflictPolicy>, bool) -> ConflictPolicy`.

- [ ] **Step 1: Write the failing test**

Append to `conflict.rs` (above `#[cfg(test)]`):

```rust
use crate::ConflictPolicy;

/// A user's answer to one conflict prompt. `Default` = bare enter = skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Default,
    Char(char),
}

/// Interpret a prompt answer into (policy, apply_to_all). Unknown keys default
/// to skip-not-all so a stray key never destroys a file.
pub fn interpret(answer: Answer) -> (ConflictPolicy, bool) {
    match answer {
        Answer::Default => (ConflictPolicy::Skip, false),
        Answer::Char('s') => (ConflictPolicy::Skip, false),
        Answer::Char('S') => (ConflictPolicy::Skip, true),
        Answer::Char('o') => (ConflictPolicy::Overwrite, false),
        Answer::Char('O') => (ConflictPolicy::Overwrite, true),
        Answer::Char('b') => (ConflictPolicy::Backup, false),
        Answer::Char(_) => (ConflictPolicy::Skip, false),
    }
}

/// Resolve a global policy from flags. `--force`/`--skip`/`--backup` are
/// mutually exclusive; the first set wins. `None` when none are set.
pub fn global_policy(force: bool, skip: bool, backup: bool) -> Option<ConflictPolicy> {
    if force {
        Some(ConflictPolicy::Overwrite)
    } else if backup {
        Some(ConflictPolicy::Backup)
    } else if skip {
        Some(ConflictPolicy::Skip)
    } else {
        None
    }
}

/// Choose the effective policy: a global flag, else `Ask` on a TTY, else `Skip`
/// (the safe non-interactive default).
pub fn resolve_policy(global: Option<ConflictPolicy>, is_tty: bool) -> ConflictPolicy {
    if let Some(p) = global {
        p
    } else if is_tty {
        ConflictPolicy::Ask
    } else {
        ConflictPolicy::Skip
    }
}
```

Add tests:

```rust
    #[test]
    fn interpret_answers() {
        assert_eq!(interpret(Answer::Default), (ConflictPolicy::Skip, false));
        assert_eq!(interpret(Answer::Char(' ')), (ConflictPolicy::Skip, false));
        assert_eq!(interpret(Answer::Char('s')), (ConflictPolicy::Skip, false));
        assert_eq!(interpret(Answer::Char('S')), (ConflictPolicy::Skip, true));
        assert_eq!(interpret(Answer::Char('o')), (ConflictPolicy::Overwrite, false));
        assert_eq!(interpret(Answer::Char('O')), (ConflictPolicy::Overwrite, true));
        assert_eq!(interpret(Answer::Char('b')), (ConflictPolicy::Backup, false));
    }

    #[test]
    fn global_policy_from_flags() {
        assert_eq!(global_policy(true, false, false), Some(ConflictPolicy::Overwrite));
        assert_eq!(global_policy(false, true, false), Some(ConflictPolicy::Skip));
        assert_eq!(global_policy(false, false, true), Some(ConflictPolicy::Backup));
        assert_eq!(global_policy(false, false, false), None);
    }

    #[test]
    fn resolve_policy_precedence() {
        assert_eq!(resolve_policy(Some(ConflictPolicy::Overwrite), false), ConflictPolicy::Overwrite);
        assert_eq!(resolve_policy(None, true), ConflictPolicy::Ask);
        assert_eq!(resolve_policy(None, false), ConflictPolicy::Skip);
    }
}
```

(Note: the closing `}` ends the `#[cfg(test)] mod tests` block — place these tests before the existing module close, i.e. merge into the same `mod tests`.)

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace conflict::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/conflict.rs
git commit -m "feat(marketplace): conflict policy flags and answer interpretation"
```

---

### Task 12: apply_copy (conflict.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/conflict.rs`

**Interfaces:**
- Produces: `pub enum Outcome`, `pub fn apply_copy(&CopyOp, ConflictPolicy, bool) -> Result<Outcome, String>`.

- [ ] **Step 1: Write the failing test**

Append to `conflict.rs` (above `#[cfg(test)]`):

```rust
/// Result of attempting one copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Installed,
    Skipped,
    BackedUp(String),
}

/// Apply one CopyOp under `policy`. When `dry_run`, report the would-be outcome
/// without touching the filesystem. `Ask` must be resolved upstream.
pub fn apply_copy(op: &CopyOp, policy: ConflictPolicy, dry_run: bool) -> Result<Outcome, String> {
    let existed = op.dst.exists();
    let outcome = match (existed, policy) {
        (true, ConflictPolicy::Skip) => Outcome::Skipped,
        (true, ConflictPolicy::Overwrite) => Outcome::Installed,
        (true, ConflictPolicy::Backup) => {
            Outcome::BackedUp(backup_name(&op.dst).display().to_string())
        }
        (true, ConflictPolicy::Ask) => {
            return Err("internal: Ask policy reached apply_copy".to_string())
        }
        (false, _) => Outcome::Installed,
    };
    if dry_run {
        return Ok(outcome);
    }
    if existed {
        match policy {
            ConflictPolicy::Skip => return Ok(Outcome::Skipped),
            ConflictPolicy::Backup => {
                let bak = backup_name(&op.dst);
                std::fs::rename(&op.dst, &bak).map_err(|e| format!("backup failed: {e}"))?;
            }
            ConflictPolicy::Overwrite => {}
            ConflictPolicy::Ask => unreachable!(),
        }
    }
    if let Some(parent) = op.dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
    }
    std::fs::copy(&op.src, &op.dst).map_err(|e| format!("copy failed: {e}"))?;
    Ok(if existed && policy == ConflictPolicy::Backup {
        outcome
    } else {
        Outcome::Installed
    })
}
```

Add tests:

```rust
    #[test]
    fn apply_installs_new_file() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a"), "new").unwrap();
        let op = CopyOp {
            src: src.path().join("a"),
            dst: dst.path().join("a"),
        };
        assert_eq!(apply_copy(&op, ConflictPolicy::Skip, false).unwrap(), Outcome::Installed);
        assert_eq!(std::fs::read_to_string(dst.path().join("a")).unwrap(), "new");
    }

    #[test]
    fn apply_skip_keeps_existing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a"), "new").unwrap();
        std::fs::write(dst.path().join("a"), "old").unwrap();
        let op = CopyOp { src: src.path().join("a"), dst: dst.path().join("a") };
        assert_eq!(apply_copy(&op, ConflictPolicy::Skip, false).unwrap(), Outcome::Skipped);
        assert_eq!(std::fs::read_to_string(dst.path().join("a")).unwrap(), "old");
    }

    #[test]
    fn apply_overwrite_replaces() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a"), "new").unwrap();
        std::fs::write(dst.path().join("a"), "old").unwrap();
        let op = CopyOp { src: src.path().join("a"), dst: dst.path().join("a") };
        assert_eq!(apply_copy(&op, ConflictPolicy::Overwrite, false).unwrap(), Outcome::Installed);
        assert_eq!(std::fs::read_to_string(dst.path().join("a")).unwrap(), "new");
    }

    #[test]
    fn apply_backup_preserves_then_replaces() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a"), "new").unwrap();
        std::fs::write(dst.path().join("a"), "old").unwrap();
        let op = CopyOp { src: src.path().join("a"), dst: dst.path().join("a") };
        let out = apply_copy(&op, ConflictPolicy::Backup, false).unwrap();
        assert!(matches!(out, Outcome::BackedUp(_)));
        assert_eq!(std::fs::read_to_string(dst.path().join("a")).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dst.path().join("a.bak")).unwrap(), "old");
    }

    #[test]
    fn apply_dry_run_writes_nothing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a"), "new").unwrap();
        let op = CopyOp { src: src.path().join("a"), dst: dst.path().join("a") };
        apply_copy(&op, ConflictPolicy::Skip, true).unwrap();
        assert!(!dst.path().join("a").exists());
    }
}
```

(Merge into the existing `mod tests`.)

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace conflict::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/conflict.rs
git commit -m "feat(marketplace): apply copy ops with skip/overwrite/backup and dry-run"
```

---

### Task 13: TUI key handling + selection state (tui.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/tui.rs`

**Interfaces:**
- Produces: `pub enum Action`, `pub fn key_to_action(crossterm::event::KeyEvent) -> Option<Action>`, `pub struct State`, `State` methods.
- Consumes: crossterm event types.

- [ ] **Step 1: Write the failing test**

Replace `tui.rs` content with:

```rust
//! Interactive multi-select UI (ratatui + crossterm). Pure logic (key mapping
//! and selection state) is separated from rendering/terminal I/O for testing.

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent};

/// A high-level action derived from a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Toggle,
    Confirm,
    Cancel,
    Up,
    Down,
    SelectAll,
    ClearAll,
}

/// Map a raw key to an action (None for ignored keys).
pub fn key_to_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char(' ') => Some(Action::Toggle),
        KeyCode::Enter => Some(Action::Confirm),
        KeyCode::Esc | KeyCode::Char('q') => Some(Action::Cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::Char('a') => Some(Action::SelectAll),
        KeyCode::Char('n') => Some(Action::ClearAll),
        _ => None,
    }
}

/// Pure selection state over a catalog of `len` items.
#[derive(Debug, Clone)]
pub struct State {
    pub selected: BTreeSet<usize>,
    pub highlight: usize,
}

impl State {
    pub fn new(len: usize) -> Self {
        let _ = len;
        Self { selected: BTreeSet::new(), highlight: 0 }
    }

    pub fn apply(&mut self, action: Action, len: usize) {
        if len == 0 {
            return;
        }
        match action {
            Action::Toggle => {
                if self.selected.contains(&self.highlight) {
                    self.selected.remove(&self.highlight);
                } else {
                    self.selected.insert(self.highlight);
                }
            }
            Action::Up => {
                if self.highlight > 0 {
                    self.highlight -= 1;
                }
            }
            Action::Down => {
                if self.highlight + 1 < len {
                    self.highlight += 1;
                }
            }
            Action::SelectAll => {
                self.selected = (0..len).collect();
            }
            Action::ClearAll => {
                self.selected.clear();
            }
            Action::Confirm | Action::Cancel => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn kc(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn keys_map_to_actions() {
        assert_eq!(key_to_action(kc(KeyCode::Char(' '))), Some(Action::Toggle));
        assert_eq!(key_to_action(kc(KeyCode::Enter)), Some(Action::Confirm));
        assert_eq!(key_to_action(kc(KeyCode::Esc)), Some(Action::Cancel));
        assert_eq!(key_to_action(kc(KeyCode::Char('q'))), Some(Action::Cancel));
        assert_eq!(key_to_action(kc(KeyCode::Down)), Some(Action::Down));
        assert_eq!(key_to_action(kc(KeyCode::Char('j'))), Some(Action::Down));
        assert_eq!(key_to_action(kc(KeyCode::Char('a'))), Some(Action::SelectAll));
        assert_eq!(key_to_action(kc(KeyCode::Char('n'))), Some(Action::ClearAll));
        assert_eq!(key_to_action(kc(KeyCode::Char('z'))), None);
    }

    #[test]
    fn toggle_and_move() {
        let mut s = State::new(3);
        s.apply(Action::Toggle, 3);
        assert_eq!(s.selected, [0].into_iter().collect::<BTreeSet<_>>());
        s.apply(Action::Down, 3);
        s.apply(Action::Toggle, 3);
        assert_eq!(s.selected, [0, 1].into_iter().collect::<BTreeSet<_>>());
    }

    #[test]
    fn select_all_then_clear() {
        let mut s = State::new(3);
        s.apply(Action::SelectAll, 3);
        assert_eq!(s.selected.len(), 3);
        s.apply(Action::ClearAll, 3);
        assert!(s.selected.is_empty());
    }

    #[test]
    fn down_clamps_at_end() {
        let mut s = State::new(2);
        s.apply(Action::Down, 2);
        s.apply(Action::Down, 2);
        assert_eq!(s.highlight, 1);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace tui::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/tui.rs
git commit -m "feat(marketplace): TUI key handling and selection state"
```

---

### Task 14: TUI render + event loop (tui.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/tui.rs`

**Interfaces:**
- Produces: `pub enum Selection { Chosen(Vec<usize>), Cancelled }`, `pub fn run_selection(&[CatalogEntry]) -> std::io::Result<Selection>`.

- [ ] **Step 1: Implement the rendering + loop**

Append to `tui.rs` (above `#[cfg(test)]`):

```rust
use crate::CatalogEntry;

/// Outcome of the interactive selection.
#[derive(Debug, Clone)]
pub enum Selection {
    Chosen(Vec<usize>),
    Cancelled,
}

/// Run the full-screen multi-select. Returns the chosen indices, or Cancelled.
/// The caller MUST have verified stdout is a TTY before calling.
pub fn run_selection(entries: &[CatalogEntry]) -> std::io::Result<Selection> {
    use crossterm::event;
    use crossterm::terminal;
    use crossterm::execute;
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Alignment, Constraint, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
    use ratatui::Terminal;

    let mut state = State::new(entries.len());
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    let _ = execute!(stdout, terminal::EnterAlternateScreen);
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let result = (|| -> std::io::Result<Selection> {
        loop {
            term.draw(|f| {
                let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(2)])
                    .split(f.size());
                let items: Vec<ListItem> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let mark = if state.selected.contains(&i) { "[*]" } else { "[ ]" };
                        let desc = e.description.clone().unwrap_or_else(|| "(no description)".to_string());
                        let tmpl = if e.template_sets.is_empty() {
                            String::new()
                        } else {
                            format!("  +templates: {}", e.template_sets.join(", "))
                        };
                        let line = Line::from(vec![
                            Span::raw(format!("{mark} {} — {desc}", e.name)),
                            Span::raw(tmpl),
                        ]);
                        ListItem::new(line)
                    })
                    .collect();
                let title = format!(
                    " steER marketplace — space:toggle  enter:confirm  a/n:all/none  q:cancel "
                );
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(title))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                f.render_stateful_widget(list, chunks[0], &mut list_state.clone().into_selected(state.highlight));

                let help = "↑/↓ or j/k move · space toggle · a all · n none · enter confirm · q/Esc cancel";
                f.render_widget(
                    Paragraph::new(help).alignment(Alignment::Center),
                    chunks[1],
                );
            })?;

            if let event::Event::Key(key) = event::read()? {
                match key_to_action(key) {
                    Some(Action::Confirm) => {
                        return Ok(Selection::Chosen(state.selected.into_iter().collect()))
                    }
                    Some(Action::Cancel) => return Ok(Selection::Cancelled),
                    Some(a) => {
                        state.apply(a, entries.len());
                        list_state.select(Some(state.highlight));
                    }
                    None => {}
                }
            }
        }
    })();

    let _ = execute!(term.backend_mut(), terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    result
}

// Helper trait to turn our index into a ListState for rendering.
trait ListStateExt {
    fn into_selected(self, idx: usize) -> ListState;
}
impl ListStateExt for ListState {
    fn into_selected(mut self, idx: usize) -> ListState {
        self.select(Some(idx));
        self
    }
}
```

- [ ] **Step 2: Build + clippy**

Run: `cargo build -p steer-marketplace && cargo clippy -p steer-marketplace -- -D warnings`
Expected: PASS. (The `.clone().into_selected(...)` call is awkward — simplify: replace `&mut list_state.clone().into_selected(state.highlight)` with constructing a fresh state each frame: `let mut ls = ListState::default(); ls.select(Some(state.highlight)); f.render_stateful_widget(list, chunks[0], &mut ls);` and delete the `ListStateExt` trait. Apply this refactor before committing.)

- [ ] **Step 3: Refactor (simplify render state)**

In the draw closure, replace the render line and remove the `ListStateExt` trait. Final render section:

```rust
                let mut ls = ListState::default();
                ls.select(Some(state.highlight));
                f.render_stateful_widget(list, chunks[0], &mut ls);
```

Remove the `let mut list_state = ...; list_state.select(Some(0));` near the top (no longer used) and the `list_state.select(...)` inside the key loop. Delete the entire `trait ListStateExt { ... }` block at the bottom.

Run: `cargo clippy -p steer-marketplace -- -D warnings` → PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/steer-marketplace/src/tui.rs
git commit -m "feat(marketplace): ratatui multi-select render and event loop"
```

---

### Task 15: install orchestrator core (install.rs)

**Files:**
- Modify: `crates/steer-marketplace/src/install.rs`

**Interfaces:**
- Produces: `pub fn install(InstallArgs) -> ExitCode`, plus `fn install_at(args, steer_dir, cwd) -> Result<Report, String>` (testable form).
- Consumes: `source::resolve`, `git::{clone, TempGuard, git_present}`, `catalog::{catalog_base, scan}`, `conflict::{plan_copies, detect_conflicts, global_policy, resolve_policy, apply_copy, Outcome}`, `tui::run_selection`, `CatalogEntry`, `CopyOp`.

- [ ] **Step 1: Write the failing test (end-to-end with a fixture marketplace)**

Replace `install.rs` content with:

```rust
//! Install orchestrator: resolve → clone → scan → select → conflict → copy →
//! summary. `install` is the CLI entry point; `install_at` is the testable core
//! that takes explicit paths.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::conflict::{
    apply_copy, detect_conflicts, global_policy, plan_copies, resolve_policy, Outcome,
};
use crate::{catalog, git, source, tui, CatalogEntry, ConflictPolicy, InstallArgs};

/// A counted summary of one install.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub installed: usize,
    pub skipped: usize,
    pub backed_up: usize,
    pub warnings: Vec<String>,
    pub dry_run: bool,
}

/// Testable core: run the install pipeline rooted at `steer_dir` (`.steer`),
/// using `registries` for name resolution and `is_tty`/selection to drive
/// selection. Interactive UI is only invoked when `is_tty` and no selection
/// flags are given.
pub fn install_at(
    args: &InstallArgs,
    steer_dir: &Path,
    registries: &[PathBuf],
    env_url: Option<&str>,
    is_tty: bool,
) -> Result<Report, String> {
    let resolved = source::resolve(args.marketplace.as_deref(), registries, env_url)?;

    let guard = git::TempGuard::new().map_err(|e| format!("temp dir failed: {e}"))?;
    git::clone(&resolved.url, args.git_ref.as_deref(), guard.path())?;

    let base = catalog::catalog_base(guard.path());
    let entries = catalog::scan(&base);
    if entries.is_empty() {
        return Ok(Report::default());
    }

    let chosen = select(args, &entries, is_tty)?;
    if chosen.is_empty() {
        return Ok(Report::default());
    }
    let chosen_entries: Vec<&CatalogEntry> = chosen.iter().map(|i| &entries[*i]).collect();

    let (ops, missing) = plan_copies(&chosen_entries, steer_dir);
    let (conflicts, clean) = detect_conflicts(&ops);
    let policy = resolve_policy(global_policy(args.force, args.skip, args.backup), is_tty);

    let mut report = Report {
        dry_run: args.dry_run,
        warnings: missing
            .iter()
            .map(|s| format!("warning: template set `{s}` referenced but absent in marketplace; workflow installed without it"))
            .collect(),
        ..Report::default()
    };

    // Clean copies always install (no conflict).
    for op in &clean {
        apply_copy(op, ConflictPolicy::Overwrite, args.dry_run)?;
        report.installed += 1;
    }
    // Conflicts resolved under `policy` (Ask resolved per-file on a TTY).
    let resolved_policy = resolve_conflicts(policy, &conflicts, is_tty, args)?;
    for op in &conflicts {
        match apply_copy(op, resolved_policy, args.dry_run)? {
            Outcome::Installed => report.installed += 1,
            Outcome::Skipped => report.skipped += 1,
            Outcome::BackedUp(_) => {
                report.backed_up += 1;
                report.installed += 1;
            }
        }
    }
    Ok(report)
}

/// Resolve which catalog indices to install, honoring flags or the TUI.
fn select(
    args: &InstallArgs,
    entries: &[CatalogEntry],
    is_tty: bool,
) -> Result<Vec<usize>, String> {
    if args.all {
        return Ok((0..entries.len()).collect());
    }
    if let Some(spec) = &args.workflows {
        let names: Vec<&str> = spec.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut out = Vec::new();
        for name in names {
            match entries.iter().position(|e| e.name == name) {
                Some(i) => out.push(i),
                None => return Err(format!("unknown workflow `{name}` in --workflows")),
            }
        }
        return Ok(out);
    }
    if !is_tty {
        return Err(
            "not a TTY and no --workflows/--all given; pass --workflows <names> or --all to install non-interactively".to_string(),
        );
    }
    match tui::run_selection(entries).map_err(|e| format!("selection UI failed: {e}"))? {
        tui::Selection::Chosen(idx) => Ok(idx),
        tui::Selection::Cancelled => Ok(Vec::new()),
    }
}

/// Resolve the effective conflict policy. If `Ask` and interactive, prompt
/// per conflict (apply-to-all short-circuits). Otherwise return the policy.
fn resolve_conflicts(
    mut policy: ConflictPolicy,
    conflicts: &[crate::CopyOp],
    is_tty: bool,
    args: &InstallArgs,
) -> Result<ConflictPolicy, String> {
    if policy != ConflictPolicy::Ask || conflicts.is_empty() || !is_tty {
        return Ok(if policy == ConflictPolicy::Ask {
            ConflictPolicy::Skip
        } else {
            policy
        });
    }
    use std::io::Write;
    for op in conflicts {
        if policy != ConflictPolicy::Ask {
            break;
        }
        print!(
            "conflict: {} exists. [s]kip / [o]verwrite / [b]ackup (S/O = all): ",
            op.dst.display()
        );
        let _ = std::io::stdout().flush();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).map_err(|e| format!("read failed: {e}"))?;
        let ans = buf.trim().chars().next().map(crate::conflict::Answer::Char).unwrap_or(crate::conflict::Answer::Default);
        let (p, all) = crate::conflict::interpret(ans);
        if all {
            policy = p;
        }
    }
    let _ = args;
    Ok(policy)
}

/// CLI entry point: resolve paths from the CWD and run.
pub fn install(args: InstallArgs) -> ExitCode {
    let steer_dir = PathBuf::from(".steer");
    let project_reg = steer_dir.join("marketplaces.toml");
    let mut registries = vec![project_reg];
    if let Some(user_reg) = source::user_registry_path() {
        registries.push(user_reg);
    }
    let env_url = std::env::var("STEER_MARKETPLACE_URL").ok();
    let is_tty = std::io::stdout().is_terminal();

    match install_at(&args, &steer_dir, &registries, env_url.as_deref(), is_tty) {
        Ok(report) => {
            print_summary(&report);
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn print_summary(report: &Report) {
    if report.dry_run {
        println!("(dry run) would install {}, skip {}, back up {}", report.installed, report.skipped, report.backed_up);
    } else {
        println!("installed {}, skipped {}, backed up {}", report.installed, report.skipped, report.backed_up);
    }
    for w in &report.warnings {
        println!("{w}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fixture_market() -> tempfile::TempDir {
        // A local git repo serving as the marketplace.
        let origin = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(origin.path().join("workflows")).unwrap();
        std::fs::create_dir_all(origin.path().join("templates/t")).unwrap();
        std::fs::write(
            origin.path().join("workflows/a.steer"),
            "@description = \"A\"\n@template = \"t\"\ntask(\"x\")\n",
        )
        .unwrap();
        std::fs::write(origin.path().join("templates/t/one.j2.md"), "body").unwrap();
        // git init + commit so it can be cloned.
        let _ = std::process::Command::new("git").current_dir(origin.path()).args(["init", "-q"]).status();
        let _ = std::process::Command::new("git").current_dir(origin.path()).args(["config", "user.email", "t@t"]).status();
        let _ = std::process::Command::new("git").current_dir(origin.path()).args(["config", "user.name", "t"]).status();
        let _ = std::process::Command::new("git").current_dir(origin.path()).args(["add", "."]).status();
        let _ = std::process::Command::new("git").current_dir(origin.path()).args(["commit", "-qm", "c"]).status();
        origin
    }

    #[test]
    fn install_all_into_fresh_project() {
        if !git::git_present() {
            eprintln!("skipping: git not present");
            return;
        }
        let origin = make_fixture_market();
        let project = tempfile::tempdir().unwrap();
        let url = origin.path().to_str().unwrap().to_string();
        let args = InstallArgs { marketplace: Some(url), all: true, ..Default::default() };
        let report = install_at(&args, &project.path().join(".steer"), &[], None, false).unwrap();
        assert_eq!(report.installed, 2, "{report:?}"); // a.steer + one.j2.md
        assert!(project.path().join(".steer/workflows/a.steer").is_file());
        assert!(project.path().join(".steer/templates/t/one.j2.md").is_file());
    }

    #[test]
    fn install_with_skip_leaves_existing() {
        if !git::git_present() {
            return;
        }
        let origin = make_fixture_market();
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join(".steer/workflows")).unwrap();
        std::fs::write(project.path().join(".steer/workflows/a.steer"), "OLD").unwrap();
        let url = origin.path().to_str().unwrap().to_string();
        let args = InstallArgs { marketplace: Some(url), all: true, skip: true, ..Default::default() };
        let report = install_at(&args, &project.path().join(".steer"), &[], None, false).unwrap();
        assert_eq!(std::fs::read_to_string(project.path().join(".steer/workflows/a.steer")).unwrap(), "OLD");
        assert!(report.skipped >= 1);
    }

    #[test]
    fn install_unknown_workflow_name_errors() {
        if !git::git_present() {
            return;
        }
        let origin = make_fixture_market();
        let project = tempfile::tempdir().unwrap();
        let url = origin.path().to_str().unwrap().to_string();
        let args = InstallArgs { marketplace: Some(url), workflows: Some("nope".to_string()), ..Default::default() };
        let err = install_at(&args, &project.path().join(".steer"), &[], None, false).unwrap_err();
        assert!(err.contains("unknown workflow `nope`"), "{err}");
    }

    #[test]
    fn install_dry_run_writes_nothing() {
        if !git::git_present() {
            return;
        }
        let origin = make_fixture_market();
        let project = tempfile::tempdir().unwrap();
        let url = origin.path().to_str().unwrap().to_string();
        let args = InstallArgs { marketplace: Some(url), all: true, dry_run: true, ..Default::default() };
        let report = install_at(&args, &project.path().join(".steer"), &[], None, false).unwrap();
        assert!(report.dry_run);
        assert!(!project.path().join(".steer/workflows/a.steer").exists());
    }

    #[test]
    fn install_non_tty_without_flags_errors() {
        if !git::git_present() {
            return;
        }
        let origin = make_fixture_market();
        let project = tempfile::tempdir().unwrap();
        let url = origin.path().to_str().unwrap().to_string();
        let args = InstallArgs { marketplace: Some(url), ..Default::default() };
        let err = install_at(&args, &project.path().join(".steer"), &[], None, false).unwrap_err();
        assert!(err.contains("not a TTY"));
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p steer-marketplace install::tests`
Expected: PASS (5 tests; the non-TTY/clone ones run where git is present).

- [ ] **Step 3: Commit**

```bash
git add crates/steer-marketplace/src/install.rs
git commit -m "feat(marketplace): install orchestrator with selection, conflicts, dry-run"
```

---

### Task 16: wire CLI subcommand (steer-cli)

**Files:**
- Modify: `crates/steer-cli/Cargo.toml`
- Modify: `crates/steer-cli/src/main.rs`

**Interfaces:**
- Consumes: `steer_marketplace::{install, InstallArgs}`.

- [ ] **Step 1: Add the dependency**

In `crates/steer-cli/Cargo.toml` `[dependencies]`, add:

```toml
steer-marketplace.workspace = true
```

- [ ] **Step 2: Add the `Install` variant + handler**

In `crates/steer-cli/src/main.rs`, extend `WorkflowAction` (after the `List` variant):

```rust
    /// Install workflows from a marketplace repository into `.steer/`.
    Install(InstallArgs),
```

Add the args struct near `WorkflowArgs`:

```rust
#[derive(Parser, Debug)]
struct InstallArgs {
    /// Marketplace source: a URL, a registered name, or omitted to use
    /// STEER_MARKETPLACE_URL.
    #[arg(long)]
    marketplace: Option<String>,
    /// Comma-separated workflow names to install (non-interactive).
    #[arg(long, value_name = "NAMES")]
    workflows: Option<String>,
    /// Install every workflow in the catalog (non-interactive).
    #[arg(long)]
    all: bool,
    /// Overwrite all conflicts without prompting.
    #[arg(long)]
    force: bool,
    /// Skip all conflicts without prompting.
    #[arg(long)]
    skip: bool,
    /// Back up all conflicts (rename to .bak) without prompting.
    #[arg(long)]
    backup: bool,
    /// Plan only; do not write, back up, or delete anything.
    #[arg(long)]
    dry_run: bool,
    /// Branch or tag to check out when cloning.
    #[arg(long, value_name = "REF")]
    ref_: Option<String>,
}
```

Add the match arm in `main` (in the `Resource::Workflow` match):

```rust
            WorkflowAction::Install(args) => run_install(args),
```

Add the handler (near `run_list`):

```rust
/// Run `steer workflow install`: delegate to the steer-marketplace crate.
fn run_install(args: InstallArgs) -> ExitCode {
    steer_marketplace::install(steer_marketplace::InstallArgs {
        marketplace: args.marketplace,
        workflows: args.workflows,
        all: args.all,
        force: args.force,
        skip: args.skip,
        backup: args.backup,
        dry_run: args.dry_run,
        git_ref: args.ref_,
    })
}
```

- [ ] **Step 3: Build, clippy, smoke**

Run: `cargo build --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS.

Run: `cargo run -q -p steer-cli -- workflow install --help`
Expected: prints help listing `--marketplace`, `--workflows`, `--all`, `--force`, `--skip`, `--backup`, `--dry-run`, `--ref`.

Run: `cargo run -q -p steer-cli -- workflow install`
Expected: non-zero exit with `error: no marketplace source configured...`.

- [ ] **Step 4: Commit**

```bash
git add crates/steer-cli/Cargo.toml crates/steer-cli/src/main.rs Cargo.lock
git commit -m "feat(cli): steer workflow install subcommand"
```

---

### Task 17: full gate

**Files:** none (verification).

- [ ] **Step 1: Format**

Run: `cargo fmt --all`
Expected: no output (formats in place).

- [ ] **Step 2: Lint**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS, zero warnings.

- [ ] **Step 3: Test**

Run: `cargo test --workspace --all-features`
Expected: PASS (existing ~130 tests + the new marketplace tests).

- [ ] **Step 4: Commit if fmt touched anything**

```bash
git add -u && git commit -m "style: cargo fmt" || echo "nothing to format"
```

---

### Task 18: documentation

**Files:**
- Modify: `README.md`
- Create: `docs/specs/workflow-install.md`
- Modify: `docs/specs/index.md`

- [ ] **Step 1: README — extend the CLI section**

In `README.md`, in the CLI code block (after the `workflow list` line), add:

```
steer workflow install [opts]   # install workflows (+ their templates) from a marketplace repo
```

After the CLI block, add a new subsection:

```markdown
### Workflow marketplace (`steer workflow install`)

`steer workflow install` fetches a marketplace repository, shows an interactive
multi-select of its workflows, and copies each selected workflow **and the
template sets it references via `@template`** into `.steer/`.

```
steer workflow install                         # uses STEER_MARKETPLACE_URL
steer workflow install --marketplace <url>
steer workflow install --marketplace <name>    # name resolved from marketplaces.toml
steer workflow install --all --force           # non-interactive: all workflows, overwrite
steer workflow install --workflows a,b --dry-run
```

Flags: `--marketplace <url|name>`, `--workflows <names>`, `--all`,
`--force`/`--skip`/`--backup` (conflict policy), `--dry-run`, `--ref <branch|tag>`.

The marketplace repository mirrors `.steer/`: a `workflows/` directory of
`*.steer` files (each with `@description`) and a `templates/` directory of
template sets. No manifest is needed — templates are resolved from each
workflow's `@template`. Any steer project pushed to git is a valid marketplace.

Named marketplaces are registered in `.steer/marketplaces.toml`:

```toml
[marketplaces.official]
url = "https://github.com/wangchen7722/steer-marketplace"
```

`install` is a **package-manager command**: unlike the agent-driven `instance`
runtime, it touches git, the terminal, and the filesystem. Installing a workflow
means an agent will later execute its instructions, so only install from sources
you trust (v1 has no signature verification).
```

- [ ] **Step 2: Behavior spec**

Create `docs/specs/workflow-install.md` summarizing the command surface (source resolution precedence, flags, conflict defaults, marketplace repo layout, exit codes), in the same Given/When/Then style as the other `docs/specs/` files. Cross-link the OpenSpec change specs under `openspec/changes/workflow-marketplace-install/specs/`.

- [ ] **Step 3: Update the docs index**

In `docs/specs/index.md`, add a one-line entry for `workflow-install.md` per the docs index convention. Verify every direct child of `docs/specs/` is listed.

- [ ] **Step 4: Verify docs build consistency**

Run: `ls docs/specs/` and confirm `index.md` lists every file/directory present.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/specs/workflow-install.md docs/specs/index.md
git commit -m "docs: document steer workflow install and marketplace layout"
```

---

## Self-review notes

- **Spec coverage:** every requirement across the four spec files maps to a task:
  - `marketplace-resolution` (5 reqs) → Tasks 2, 3, 4.
  - `workflow-install` (8 reqs) → Tasks 5, 6, 7, 8, 15, 16 (clone/temp, git check, scan/base, workflow+templates, `.steer/` creation, `--workflows`/`--all`, `--dry-run`, summary/exit codes).
  - `interactive-install-selection` (4 reqs) → Tasks 13, 14, 15 (TTY gate, row content, keybindings, empty-confirm).
  - `install-conflict-resolution` (5 reqs) → Tasks 9, 10, 11, 12, 15.
- **Type consistency:** `CatalogEntry`, `ConflictPolicy`, `CopyOp`, `InstallArgs` defined once in `lib.rs` and reused unchanged across tasks; `Outcome`, `Answer`, `Selection`, `Action`, `State`, `Report` defined in their owning module and referenced by exact name.
- **No placeholders:** every step contains real code or exact commands; the two intentional "refactor before commit" notes (Task 6 duplicate write, Task 14 list-state) give exact replacements.
```
```
```

---

End of plan.
