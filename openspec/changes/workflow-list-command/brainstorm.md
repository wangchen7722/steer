# Brainstorm — `steer workflow list`

> Raw capture of collaborative exploration for change `workflow-list-command`.
> This is the divergent/convergent thinking that feeds `proposal.md`, `specs/`,
> `design.md`, `tasks.md`, and `plan.md`. It is exploratory, not a spec.

## The ask, in one line

A `steer workflow list` command that shows every workflow under
`.steer/workflows/` by **name** and by a new **`@description`** metadata line,
so an agent or user can see what workflows exist and what each does without
opening each `.steer` file.

## What exists today (the lay of the land)

- `.steer/workflows/` is the de-facto catalog directory. The discovery logic in
  `crates/steer-cli/src/main.rs::resolve_workflow` already looks there as a
  fallback when a `<workflow>` path arg is not a file — so the directory *is*
  the registry, there is just no way to enumerate it.
- `@`-directives are first-class: `@template = "..."` and `@context = "..."`
  are parsed as `Stmt::Meta` and extracted in **three** near-identical places:
  - `crates/steer-core/src/storage.rs::extract_meta` (instance start),
  - `crates/steer-core/src/simulate.rs::apply_static_meta` (dry-run),
  - `crates/steer-core/src/vm.rs::apply_meta` (runtime).
  All three use the same trick: `eval_literal(value).render()`, empty → `None`.
- `@context` is shown by `instance start` / `instance status` (runtime-facing).
  There is **no** directive whose purpose is "describe me in a catalog." `@context`
  is close, but it is written for the running instance, not for a one-line
  listing, and reusing it would overload its meaning.

## Questions worth pulling apart

### Q1. What does `list` actually scan?

Default is clearly `.steer/workflows/` (the user said "默认"). Open question: do
we allow an override? Leaning **yes** — an optional `[dir]` positional, defaulting
to `.steer/workflows/`. It costs one `Option<PathBuf>` and matches the spirit of
"默认" (a default you can step away from). Recursion: **no** — stay flat, because
`resolve_workflow` does a flat lookup and the catalog is flat today. Subdirectories
are ignored. (If nested workflows ever appear, that is a separate change.)

### Q2. What is a workflow's "name"?

The **file stem**. `openspec-propose.steer` → `openspec-propose`. This is exactly
the token you pass to `instance start openspec-propose ...`, so the name `list`
prints is the name you act on. No separate `@name` directive — the filename *is*
the identity, consistent with how discovery already works.

### Q3. How is the description obtained?

Parse the file and pull the top-level `@description = "..."` `Meta` value, then
`eval_literal(value).render()` — i.e. **mirror the `@context`/`@template` path
exactly**. Empty string → `None` (same convention). Interpolations like
`@description = "hi {x}"` would render to a `{x}` placeholder because there is no
runtime scope at listing time; that is acceptable and identical to how `@context`
behaves under `simulate`. Document that `@description` is meant to be a plain
literal.

### Q4. Where does the extraction logic live, and how much runtime do we touch?

This is the real design fork:

- **Option A — extend `WorkflowMeta`.** Add `description: Option<String>` to
  `WorkflowMeta`, recognise it in all three extraction sites, expose a public
  `extract_meta`. Most "uniform" (every `@`-directive flows through the same
  struct). Cost: touches the runtime path (storage/simulate/vm) for a value that
  is **never used at runtime**, and entrenches the existing three-way
  duplication of the template/context handling.
- **Option B — one dedicated pure function.** A `pub fn workflow_description(&Module)
  -> Option<String>` that `list` calls; the runtime path is **untouched**.
  Description has no runtime effect, so it should not live in runtime state.
  Lowest risk, smallest diff, easiest to test. The only downside is a small loop
  that overlaps `extract_meta`'s loop — but it answers a different question
  (description-only) so the overlap is fine.

**Leaning hard toward Option B.** YAGNI: if description later needs to show up in
`status`/`start`, promote it then. Keep this change additive and isolated.

### Q5. Output shape?

A simple two-column table, sorted alphabetically by name, name left-padded to the
longest name so descriptions line up:

```
os-bugfix          Fix an OS-domain bug end-to-end.
openspec-apply     Apply an openspec change: implement + verify.
openspec-propose   Propose an openspec change: brainstorm → plan.
```

Machine-readable output (`--json`) is explicitly **out of scope** — humans and
agents both read this fine as text.

### Q6. What about a workflow with no `@description`, or one that won't parse?

`list` must never crash on a bad catalog entry. Decisions:

- **No `@description`** → print the name with a `(no description)` placeholder so
  the column is uniform and the absence is visible (not a silently short line).
- **Empty `@description = ""`** → treated as absent (`None`), same as `@context`.
- **Unparseable `.steer` file** → still list it by name, with an `(unparseable)`
  marker, so a broken file is discoverable rather than silently dropped.
- **Unreadable file** → `(unreadable)` marker. (Rare; mostly defensive.)
- **Directory missing or empty** → print `(no workflows in <dir>)` and exit `0`.
  Listing an empty catalog is success, not an error.

### Q7. Does `@description` change any existing behavior?

No. It is a new, optional directive. Unknown `@`-keys are already silently
ignored by the three extraction sites, so a workflow with `@description` runs,
validates, and simulates identically to one without. Purely additive — no
breaking change, no migration.

### Q8. Capability modeling — one capability or two?

The ask has two parts: the `list` command and the `@description` directive. But
the directive exists *only* to feed `list` (it has no other consumer). Coupling
them under a single capability **`workflow-listing`** keeps the proposal/spec
coherent: "you can list workflows and they carry a one-line description." Splitting
would create a `workflow-metadata` capability with a single, listing-only field —
over-modeling. **One capability.**

## Tentative conclusions (to be ratified in proposal/design)

1. `steer workflow list [dir]`, default `.steer/workflows/`, flat scan, sorted.
2. Name = file stem; description = top-level `@description` via
   `eval_literal().render()`, empty → absent.
3. Extraction = a new pure `steer_core::workflow_description(&Module)`; **runtime
   path untouched** (Option B).
4. Robust output: name always shown; `(no description)` / `(unparseable)` /
   `(unreadable)` placeholders; `(no workflows in <dir>)` when empty; exit 0.
5. One capability `workflow-listing`; additive, non-breaking.
6. Seed the three shipped workflows (`openspec-propose`, `openspec-apply`,
   `os-bugfix`) with `@description` so the feature is useful out of the box.

## Open questions to confirm with the user

- Is the optional `[dir]` override wanted, or should `list` be hard-wired to
  `.steer/workflows/`? (Proposal assumes the optional override; trivial to drop.)
- Should `@description` also surface in `instance status` output later? (Out of
  scope for this change; flagged as a non-goal.)
