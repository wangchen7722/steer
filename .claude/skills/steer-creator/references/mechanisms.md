# Steer Mechanisms

## Template System

### What Templates Do

Templates separate **what to say** (workflow logic) from **how to say it**
(prompt format). Each action node (`task`, `ask`, `command`, `collect`, `judge`,
`print`) renders through a `.j2.md` template file. You can customize these
templates to change how instructions are presented to the agent, without
modifying the workflow logic.

### Template Resolution Priority

When an action node is rendered, templates are resolved in this order:

1. `.steer/templates/<@template value>/<callee>.j2.md`: active directory (set by `@template`)
2. `.steer/templates/default/<callee>.j2.md`: default directory
3. Built-in fallback template (hardcoded in steer)
4. Generic task-like template (minimal formatter)

If a template file is found, it is used; otherwise the next level is tried.

### Template File Format (`.j2.md`)

A template file is Markdown + Jinja2, optionally with YAML front-matter:

```yaml
---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the following:
  <check>{{ check }}</check>
  Inspect the work and confirm the condition holds.
---
Follow the instruction below{% if return %} and report back{% endif %}.
<instruction>{{ instruction }}</instruction>
{% if return %}<report>Report the result via `steer instance set {{ steer_instance }} {{ steer_target }} <value>`, where <value> is the {{ return }}.</report>
{% endif %}{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}<rule>Execute only this instruction.</rule>
```

#### Front-matter: `parameter` Section

Each line declares a parameter the template accepts:

```
name: type[, required][, default=value]
```

- **type**: `string`, `bool`, `list`, `none`, `bool` (intrinsic boolean for `judge`)
- **required**: marks the parameter as mandatory
- **default=**: declarative default (e.g. `default="result"`, `default=true`)

#### Front-matter: `on_check` Section

A Jinja2 template for rendering the check instruction. Can be inline or a YAML
literal block (`on_check: |` + indented lines).

Available variables in `on_check`:
- `{{ check }}`: the evaluated `check=` argument value
- `{{ instruction }}`, `{{ return }}`, `{{ produce }}`: other call arguments
- `{{ steer_instance }}`, `{{ steer_target }}`: runtime context

The `<report>` section with `steer instance set ... checked` commands is
**always auto-appended by the VM** and must NOT be included in the `on_check`
template.

#### Without Front-matter

The entire file is the body, with a minimal formatter (requires `instruction`
as first positional).

### Jinja2 Subset

The template engine supports a minimal Jinja2 subset:

| Feature | Syntax |
|---------|--------|
| Variable interpolation | `{{ name }}` |
| Conditional block | `{% if name %}...{% else %}...{% endif %}` |
| Loop over list | `{% for x in list %}...{% endfor %}` |

**Not supported:** whitespace control (`{%-`), `elif`, nested blocks, filters,
macros, includes, `extends`/`block`.

### Template Context Variables

Available in the template body:

| Variable | Description |
|----------|-------------|
| `{{ instruction }}` | The first positional argument (task description) |
| `{{ steer_instance }}` | The run instance name |
| `{{ steer_target }}` | The variable receiving the result |
| `{{ return }}` | The expected value format (when assigned) |
| `{{ produce }}` | List of files to produce |
| `{{ check }}` | Not exposed to body (handled by `on_check`) |

### Creating Custom Templates

1. Create a directory under `.steer/templates/<name>/`.
2. Add `.j2.md` files for the action nodes you want to customize.
3. Set `@template = "<name>"` in the workflow to activate.
4. Only the nodes you override are used from your directory; others fall back
   to `default/` → built-in.

Example: `.steer/templates/planning/task.j2.md` customizes only the `task`
prompt for the "planning" phase, while `ask`, `command`, etc. still use the
default templates.

### Custom Callees (Phase-Specific Commands)

Any identifier works as a callee, not just the built-in nodes (`task`,
`ask`, `command`, `collect`, `judge`, `print`). When a call renders, steer
looks for `<callee>.j2.md` in the active `@template` directory, then
`default/`, then falls back to a generic task-like template. An unknown
callee is never an error — it just uses the generic fallback. `validate`
accepts any callee name.

This lets each workflow phase become its own "command" backed by a
dedicated template. The fixed framing for a phase — which skill to invoke,
what output rules apply, what content skeleton to follow — moves into the
template instead of being repeated in every `instruction` string:

```steer
@template = "openspec-superpowers"
brainstorm("Explore the change collaboratively.",      // -> brainstorm.j2.md
           produce=["openspec/changes/{change}/brainstorm.md"],
           check="Confirm brainstorm.md holds verbatim output")
proposal("Establish why this change is needed.", ...)  // -> proposal.j2.md
```

The `instruction` string then carries only the phase-specific dynamic
content; the `<phase>.j2.md` carries everything fixed.

### Template Mechanism Gotchas

Two behaviors catch authors of custom templates. Knowing them up front
saves real debugging time.

**`return` is only injected into the template context when the call is
assigned to a variable.** In `build_context`, a `return=` argument is
skipped for bare calls (calls with no receiver): the condition is
`name == "return" && into.is_none()`. The reasoning is that `return`
describes how to report a value, and a value-reporting prompt only makes
sense when there is a variable to receive it.

What this means in practice:

- A bare `mynode("...", return="...")` silently drops `return`: `{{ return }}`
  renders empty, and the VM emits no `<report>` block.
- To have an agent report a value back, assign the call:
  `report = mynode("...", return="...")`.
- For a pure `check=` quality gate where there is no value to report, use a
  bare call **without** `return=`. The check flow advances via `set
  checked`, which needs no `<report>` block.

**`validate` does not see `@template`.** `steer workflow validate`
resolves templates through `resolve_template()` (no meta), which looks only
in `default/` — never in the `@template` directory. So a custom callee that
declares `return: string` in `openspec-superpowers/returning.j2.md` looks,
to `validate`, like an unknown callee with no `return` spec. Assigning its
result then fails with `produces no value and cannot be assigned`, even
though at runtime the template is found and `return` works fine.

Two ways around this:

- Model the step as a bare call with a `check=` gate (no assignment, no
  `return`). `validate` is satisfied, and the check flow handles
  advancement.
- Or accept the false positive and rely on `simulate`, which **does**
  resolve `@template`, to confirm the template is found and renders
  correctly. For custom callees, `simulate` is the authoritative check;
  `validate`'s blind spot is a known limitation.

---

## Check Mechanism

### Three Check Kinds

When the agent calls `steer instance check`, the VM dispatches based on the
nature of the current op:

| Kind | When | Behavior |
|------|------|---------|
| **Auto** | No `check`, no assignment target (e.g. bare `print`) | Advances immediately |
| **Value** | Assigned to a variable, no `check` (e.g. `x = ask(...)`) | Returns `pending` until value is set, then advances |
| **Checked** | Has `check=` argument | Two-phase: render check instruction → agent verifies → report via `set checked` |

### Checked Flow (Step-by-step)

1. Agent calls `steer instance step` → gets the task instruction.
2. Agent executes the task.
3. Agent calls `steer instance check` → gets the **check instruction** (rendered
   from `on_check` template + auto-appended `<report>` section).
4. Agent performs the verification.
5. Agent reports the result:

   ```bash
   # Pass:
   steer instance set <name> checked true
   # or with explicit JSON:
   steer instance set <name> checked '{"passed":true}'

   # Fail (reason is required):
   steer instance set <name> checked '{"passed":false,"reason":"why it failed"}'
   ```

6. Agent calls `steer instance check` again:
   - If passed → `advanced` (PC moves forward).
   - If failed → `failed` (PC stays, retry mechanism kicks in).

### Retry Mechanism

When a check fails:

1. `failure_reason` is stored in `StepState`.
2. `retry_count` is incremented.
3. PC stays on the same instruction.
4. Next `step` appends retry context:

   ```
   Previous verification failed (retry #N):
   <reason>

   Retry the task and address the failure before checking again.
   ```

5. There is **no built-in retry limit**. Use `loop ... until` with a counter
   for bounded retries.

### `judge` vs `check`

These are **orthogonal mechanisms**:

- **`judge`**: Boolean judgment that returns a value into a variable. Used in
  conditions (`until passed`). No retry; it just records the answer.
- **`check`**: Step-level verification with unbounded retry. Used when you need
  "do this until it is correct."

Use `judge` for decisions; use `check` for quality gates.

---

## Context Mechanism

### Execution Context (`context.json`)

The full execution state is serialized to `.steer/instances/<name>/context.json`:

| Field | Description |
|-------|-------------|
| `pc` | Program counter (index into IR) |
| `status` | `Running`, `Complete`, or `Halted(String)` |
| `vars` | Current scope's variables |
| `frames` | Call stack (for `func`/`return`) |
| `steps` | Per-agent-op state, keyed by PC |
| `meta` | Runtime metadata from `@` directives |

### `@context` Directive

```steer
@context = "bug-fix workflow for login issues"
```

Sets a workflow-level context description. When set:
- `steer instance start <wf> <name>` shows the description after "started".
- `steer instance status <name>` shows the description.

This gives the agent a high-level understanding of what the workflow does,
without reading the entire `.steer` file.

### `@template` Directive

```steer
@template = "planning"
```

Switches the active template directory. Subsequent action nodes resolve templates
from `.steer/templates/planning/<callee>.j2.md` first. Set `"default"` or an
empty string to revert.

This enables multi-phase workflows with distinct prompt styles:

```steer
@template = "planning"
task("create proposal outline", ...)    # uses planning/task.j2.md

@template = "default"
task("implement the proposal", ...)     # uses default/task.j2.md
```

---

## Control Flow Details

### IR (Intermediate Representation)

The AST is lowered to a flat instruction stream. The PC is simply an index into
this vector:

| Instruction | Purpose |
|-------------|---------|
| `SetMeta` | Update `@template` / `@context` at runtime |
| `AgentOp` | An action node; pauses for agent |
| `Assign` | Evaluate expression, bind to variable |
| `JumpIfFalse` | Conditional jump (used by `if`, `loop ... until`) |
| `Jump` | Unconditional jump |
| `ForInit` | Initialize for-loop hidden iterator slot |
| `ForIter` | Pop next element or jump to `end` |
| `Call` | Call user function |
| `Return` | Return from function or halt at top level |
| `Halt` | End of program |

Functions are emitted after `Halt`, reachable only via `Call`.

### Scope and Variables

- Variables live in `ctx.vars` (a `HashMap<String, Value>`).
- Assignment (`x = expr`) writes to the current scope.
- `for` loops use a hidden iterator slot (`__for_N`) internally; the original
  list variable is not consumed.
- `func` saves and restores the caller's variables on the call stack (`Frame`).

### Halt and Error

- A top-level `return` halts the workflow with status `Complete`.
- `steer instance error <name> "<reason>"` halts with status `Halted(reason)`.
- The agent should use `error` when it genuinely cannot make progress, not as
  a shortcut.
