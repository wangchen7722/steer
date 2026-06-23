# `workflow-list-command` Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development`
> to implement this plan task-by-task. (That skill is not installed in this
> session, so implement the tasks directly in dependency order.)

**Goal:** Add `steer workflow list [dir]` that enumerates `*.steer` workflows and
prints each name with its `@description`, and introduce the optional
`@description` directive.

**Architecture:** A pure extractor `steer_core::workflow_description(&Module)`
sits beside `extract_meta`; the CLI scans a directory, parses each file, calls
the extractor, and prints a sorted two-column table. The runtime path is
untouched. Additive only.

**Tech Stack:** Rust 2021, clap v4 (derive), existing `steer_syntax::parse` and
`eval_literal`/`Value::render`. No new dependencies.

**Files:**

| Action | Path | Responsibility |
|---|---|---|
| Modify | `crates/steer-core/src/storage.rs` | Add `workflow_description`; add unit tests |
| Modify | `crates/steer-core/src/lib.rs` | Re-export `workflow_description` |
| Modify | `crates/steer-cli/src/main.rs` | Add `WorkflowAction::List` + `run_list` |
| Modify | `crates/steer-cli/tests/cli.rs` | Integration tests for `list` |
| Modify | `.steer/workflows/{openspec-propose,openspec-apply,os-bugfix}.steer` | Seed `@description` |
| Modify | `README.md` | Document the command + directive |
| Create | `docs/specs/workflow-listing.md` | BDD behavior spec |
| Modify | `docs/specs/cli.md`, `docs/specs/index.md` | Recognize `list`; index the new spec |

---

### Task 1: `workflow_description` extractor (core)

**Files:**
- Modify: `crates/steer-core/src/storage.rs` (add function after `extract_meta`, ~line 91; add tests in the `tests` module)
- Test: `crates/steer-core/src/storage.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `crates/steer-core/src/storage.rs`:

```rust
#[test]
fn workflow_description_extracts_directive() {
    let m = steer_syntax::parse("@description = \"a catalog entry\"\ntask(\"x\")\n").unwrap();
    assert_eq!(workflow_description(&m), Some("a catalog entry".to_string()));
}

#[test]
fn workflow_description_absent_is_none() {
    let m = steer_syntax::parse("task(\"x\")\n").unwrap();
    assert_eq!(workflow_description(&m), None);
}

#[test]
fn workflow_description_empty_is_none() {
    let m = steer_syntax::parse("@description = \"\"\ntask(\"x\")\n").unwrap();
    assert_eq!(workflow_description(&m), None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p steer-core workflow_description`
Expected: FAIL — `cannot find function workflow_description in module storage`.

- [ ] **Step 3: Write minimal implementation**

Add immediately after `extract_meta` in `crates/steer-core/src/storage.rs`:

```rust
/// Extract the top-level `@description = "..."` directive from a parsed
/// workflow module. Mirrors [`extract_meta`]: the literal value is rendered to
/// text and an empty result is treated as absent (`None`). Used by
/// `steer workflow list` to annotate catalog entries; `@description` has no
/// runtime effect.
pub fn workflow_description(module: &steer_syntax::Module) -> Option<String> {
    for s in &module.body {
        if let steer_syntax::ast::Stmt::Meta { key, value } = &s.value {
            if key == "description" {
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

Then re-export from `crates/steer-core/src/lib.rs` by extending the existing
`storage` re-export line:

```rust
pub use storage::{load_context, load_ir, save_context, start_instance, workflow_description, InstanceError};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p steer-core workflow_description`
Expected: PASS — 3 tests pass.

---

### Task 2: `workflow list` CLI subcommand

**Files:**
- Modify: `crates/steer-cli/src/main.rs` (`WorkflowAction` enum ~line 36; dispatch in `main` ~line 83; new `run_list`)
- Test: `crates/steer-cli/tests/cli.rs` (integration tests)

- [ ] **Step 1: Write the failing tests**

Add a helper and four tests to `crates/steer-cli/tests/cli.rs`:

```rust
/// Build a temp working dir whose `.steer/workflows/` holds the given
/// `(name, content)` `.steer` files, for `workflow list` tests.
fn make_workflows_dir(suffix: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("steer-list-{suffix}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let dir = tmp.join(".steer").join("workflows");
    std::fs::create_dir_all(&dir).expect("make workflows dir");
    for (name, content) in files {
        std::fs::write(dir.join(format!("{name}.steer")), content).expect("write workflow");
    }
    tmp
}

#[test]
fn list_shows_workflows_with_descriptions() {
    let tmp = make_workflows_dir(
        "basic",
        &[
            ("alpha", "@description = \"Alpha workflow\"\nprint(\"a\")\n"),
            ("beta", "print(\"b\")\n"),
        ],
    );
    let out = steer()
        .args(["workflow", "list"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alpha"), "stdout: {stdout}");
    assert!(stdout.contains("Alpha workflow"), "stdout: {stdout}");
    assert!(stdout.contains("beta"), "stdout: {stdout}");
    assert!(stdout.contains("(no description)"), "stdout: {stdout}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn list_honors_custom_dir() {
    let tmp = std::env::temp_dir().join(format!("steer-list-cwd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let custom = tmp.join("custom");
    std::fs::create_dir_all(&custom).expect("make custom dir");
    std::fs::write(
        custom.join("only.steer"),
        "@description = \"only here\"\nprint(\"x\")\n",
    )
    .expect("write workflow");
    // default dir is absent -> no workflows
    let out = steer()
        .args(["workflow", "list"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(String::from_utf8_lossy(&out.stdout).contains("no workflows"));
    // explicit custom dir -> lists the one workflow
    let out = steer()
        .args(["workflow", "list", "custom"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("only"), "stdout: {stdout}");
    assert!(stdout.contains("only here"), "stdout: {stdout}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn list_missing_dir_reports_no_workflows() {
    let tmp = std::env::temp_dir().join(format!("steer-list-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("make tmp dir");
    let out = steer()
        .args(["workflow", "list"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("no workflows"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn list_marks_unparseable_file() {
    let tmp = make_workflows_dir("broken", &[("broken", "x =\n")]);
    let out = steer()
        .args(["workflow", "list"])
        .current_dir(&tmp)
        .output()
        .expect("run steer");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("broken"), "stdout: {stdout}");
    assert!(stdout.contains("(unparseable)"), "stdout: {stdout}");
    let _ = std::fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p steer-cli list_`
Expected: FAIL — `error: unrecognized subcommand list` (clap rejects unknown subcommand).

- [ ] **Step 3: Write minimal implementation**

In `crates/steer-cli/src/main.rs`, add the variant to `WorkflowAction` (after `Simulate`):

```rust
    /// List workflows in a directory (default `.steer/workflows/`), printing each
    /// workflow's name with its `@description`.
    List {
        /// Directory to scan for `*.steer` files. Defaults to `.steer/workflows/`.
        dir: Option<PathBuf>,
    },
```

Wire dispatch in `main`'s `Resource::Workflow(w) => match w.action { ... }` arm:

```rust
            WorkflowAction::List { dir } => run_list(dir.as_deref()),
```

Add the handler (e.g. near `run_simulate`):

```rust
/// Run `steer workflow list [dir]`: enumerate `*.steer` workflows and print
/// each name with its `@description`. Defaults to `.steer/workflows/`.
///
/// Never fails on a bad entry: a missing description prints `(no description)`,
/// an unparseable file prints `(unparseable)`, an unreadable file prints
/// `(unreadable)`, and a missing/empty directory prints
/// `(no workflows in <dir>)` — all exit successfully.
fn run_list(dir: Option<&Path>) -> ExitCode {
    let dir = dir.unwrap_or_else(|| Path::new(".steer/workflows"));
    let mut rows: Vec<(String, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("steer") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            rows.push((stem.to_string(), read_description(&path)));
        }
    }
    if rows.is_empty() {
        println!("(no workflows in {})", dir.display());
        return ExitCode::SUCCESS;
    }
    // File stems are unique within a directory, so tuple ordering sorts by name.
    rows.sort();
    let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, desc) in &rows {
        println!("{name:<width$}  {desc}");
    }
    ExitCode::SUCCESS
}

/// Read and parse `path`, returning its `@description` text or a placeholder.
fn read_description(path: &Path) -> String {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return "(unreadable)".to_string(),
    };
    match steer_syntax::parse(&src) {
        Ok(module) => steer_core::workflow_description(&module)
            .unwrap_or_else(|| "(no description)".to_string()),
        Err(_) => "(unparseable)".to_string(),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p steer-cli list_`
Expected: PASS — 4 tests pass.

---

### Task 3: Seed `@description` in shipped workflows (no test — content)

**Files:**
- Modify: `.steer/workflows/openspec-propose.steer`, `openspec-apply.steer`, `os-bugfix.steer`

- [ ] **Step 1: Add a one-line `@description` next to the existing `@context`/`@template`**

Suggested text (adjust to match each workflow's purpose):

```text
# openspec-propose.steer (after @context = ...)
@description = "Spec-driven propose phase: brainstorm, proposal, specs, design, tasks, plan."

# openspec-apply.steer
@description = "Spec-driven apply phase: implement an openspec change and verify it."

# os-bugfix.steer
@description = "End-to-end OS-domain bugfix: reproduce, root-cause, fix, verify."
```

- [ ] **Step 2: Verify they still validate**

Run: `for w in openspec-propose openspec-apply os-bugfix; do steer workflow validate "$w"; done`
Expected: each prints `…/workflows/<w>.steer: OK`.

---

### Task 4: Docs (no test — content)

**Files:**
- Modify: `README.md`; Create `docs/specs/workflow-listing.md`; Modify `docs/specs/cli.md`, `docs/specs/index.md`

- [ ] **Step 1: README CLI table + directive mention**

Add to the CLI command list in `README.md`:

```
steer workflow list [dir]       # list workflows + their @description (default: .steer/workflows/)
```

In the language/metadata area, note `@description` alongside `@template`/`@context`
as an optional, listing-oriented directive.

- [ ] **Step 2: Behavior spec**

Create `docs/specs/workflow-listing.md` with `## Scenario: …` / `- **WHEN** …` /
`- **THEN** …` blocks mirroring `specs/workflow-listing/spec.md` (default dir,
custom dir, name = file stem, description shown, absent → placeholder, empty →
absent, unparseable → marker, missing dir → notice, runtime-inert).

- [ ] **Step 3: cli.md + index.md**

In `docs/specs/cli.md`, extend the "subcommands are recognized" scenario to
include `workflow list`. In `docs/specs/index.md`, add a one-line entry for the
new `workflow-listing.md` (docs index convention).

---

### Task 5: Full verification

- [ ] **Step 1: Format**

Run: `cargo fmt --all -- --check`
Expected: clean (no diff).

- [ ] **Step 2: Lint**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: clean (no warnings).

- [ ] **Step 3: Test**

Run: `cargo test --workspace --all-features`
Expected: all tests pass (existing + new).

- [ ] **Step 4: Manual smoke test**

Run: `steer workflow list` (from the repo root)
Expected output similar to:

```
os-bugfix          End-to-end OS-domain bugfix: reproduce, root-cause, fix, verify.
openspec-apply     Spec-driven apply phase: implement an openspec change and verify it.
openspec-propose   Spec-driven propose phase: brainstorm, proposal, specs, design, tasks, plan.
```
