# OpenSpec Specs Index

Behavior specifications (BDD, Given/When/Then) for the steer tool, organized by
implementation layer. Each subfolder holds one capability's `spec.md`. Descend
into the one that matches the feature in question.

## Build and CLI

| Spec | Description |
|------|-------------|
| [build-and-lint-policy](./build-and-lint-policy/spec.md) | Workspace-wide build and lint configuration that gates every contribution (lint levels, edition, release profile, formatting) as a contract. |
| [cli-command-surface](./cli-command-surface/spec.md) | The complete externally observable `steer` command tree — subcommand names, arguments, stdout strings, exit codes — the primary compatibility surface. |
| [cli-error-reporting](./cli-error-reporting/spec.md) | CLI error-reporting convention: `error:`-prefixed stderr, source-located diagnostics, FAILURE exit, clean stdout for agents/scripts. |

## DSL Front-End

| Spec | Description |
|------|-------------|
| [dsl-tokenization](./dsl-tokenization/spec.md) | Closed lexical token vocabulary and per-character classification rules of the `.steer` DSL. |
| [dsl-newline-handling](./dsl-newline-handling/spec.md) | Line-oriented statement-termination model: when `\n` becomes a `Newline` token vs. insignificant whitespace. |
| [dsl-string-literals](./dsl-string-literals/spec.md) | Double-quoted string literals: verbatim/escape segments, fixed escape set, `{...}` interpolation, single-line constraint. |
| [dsl-expression-grammar](./dsl-expression-grammar/spec.md) | Expression grammar: operator-precedence ladder, call-argument grammar (positional then named), primary forms, reserved word-operators. |
| [dsl-statement-grammar](./dsl-statement-grammar/spec.md) | Statement layer: declarative forms (meta, assignment, call) and block forms (if/elseif/else, loop/until, for/in, func, return). |
| [dsl-source-spans-and-errors](./dsl-source-spans-and-errors/spec.md) | Byte-offset span model on every lexical/AST node and the unified single-error lex/parse error categories. |

## Runtime and Interpreter

| Spec | Description |
|------|-------------|
| [value-system-and-operators](./value-system-and-operators/spec.md) | Typed runtime `Value` system (`Null|Bool|Int|Float|Str|List|Object`), operator-semantic table, and typed-JSON coercion. |
| [ir-lowering](./ir-lowering/spec.md) | `lower` compiles a parsed `Module` into a flat, index-resumable `Vec<Instr>`; control flow becomes explicit jumps. |
| [interpreter-execution](./interpreter-execution/spec.md) | `step` advances the program counter through the lowered instructions, pausing at agent operations and halting on eval errors. |
| [validation-semantic-checks](./validation-semantic-checks/spec.md) | Load-time static validator over a parsed `Module` returning every `Diagnostic` at once; shape and placement rules only. |
| [runtime-check-gate](./runtime-check-gate/spec.md) | `check` three-mode verification gate (auto/value/checked), pass-consumed-once, malformed-`checked` rejection, report-command rendering. |
| [vm-system-messages](./vm-system-messages/spec.md) | System messages steer emits itself (check report, retry context, start/status output) with single-brace placeholder syntax. |

## Instance Lifecycle

| Spec | Description |
|------|-------------|
| [instance-lifecycle](./instance-lifecycle/spec.md) | Run-state machine across CLI invocations: `Running` → terminal `Complete` or `Halted(reason)` that sticks until a fresh `start`. |
| [instance-persistence](./instance-persistence/spec.md) | On-disk layout under `.steer/instances/<name>/` and the pinned `context.json` schema; atomic-staging resume. |
| [instance-name-validation](./instance-name-validation/spec.md) | Rejects names that could escape `.steer/instances/`, confining each to a single verbatim path segment before filesystem access. |

## Template System

| Spec | Description |
|------|-------------|
| [template-substitution-syntax](./template-substitution-syntax/spec.md) | Minimal Jinja2-subset template language: `{{ name }}`, `{% if/else/endif %}`, `{% for/endfor %}`, truthiness and missing-variable rules. |
| [template-file-format](./template-file-format/spec.md) | `.j2.md` authoring format: optional YAML front matter (parameter schema, `on_check`), recognized types/modifiers, `return` semantics. |
| [template-resolution-and-loading](./template-resolution-and-loading/spec.md) | Four-tier callee→template resolution: active `@template` dir → `default/` → built-in fallback → generic task-like fallback, plus caching. |
| [template-context-binding](./template-context-binding/spec.md) | How call arguments and runtime metadata bind to the Jinja2 render context: `steer_target`/`steer_instance`, instruction binding, `check`/bare-`return` suppression. |
| [builtin-node-templates](./builtin-node-templates/spec.md) | Shipped template content for the six default nodes (task, ask, command, collect, print, judge) and their value-source/produce conventions. |

## Workflows and Discovery

| Spec | Description |
|------|-------------|
| [workflow-directive-extraction](./workflow-directive-extraction/spec.md) | How top-level `@template`/`@context`/`@description` directives extract into persisted runtime metadata and a non-persisted catalog description. |
| [workflow-discovery-and-listing](./workflow-discovery-and-listing/spec.md) | `<workflow>` argument resolution for `instance start`/`validate`/`simulate` and `steer workflow list` enumeration output. |
| [workflow-simulation](./workflow-simulation/spec.md) | `steer workflow simulate` static projection of the instruction trace: source-order walk, render every call once, no execution. |

## Bundled `.steer` Workflows

| Spec | Description |
|------|-------------|
| [openspec-propose-workflow](./openspec-propose-workflow/spec.md) | `openspec-propose` workflow: front half of a spec-driven change — brainstorm, proposal, specs, design, tasks, plan — then pause for review. |
| [openspec-apply-workflow](./openspec-apply-workflow/spec.md) | `openspec-apply` workflow: back half — execute the plan, update task checkboxes, verify against a seven-check list before archive. |
| [openspec-generate-specs-workflow](./openspec-generate-specs-workflow/spec.md) | `openspec-generate-specs` workflow: auto-generate/refresh main specs from code, docs, and git history with a closed review-refine coverage loop. |
