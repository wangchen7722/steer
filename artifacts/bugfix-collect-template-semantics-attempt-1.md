# Bugfix: collect template semantics — attempt 1

Bug: `collect-template-semantics`
Status: fixed

## Problem

`collect` is a value-producing agent op. Per `IDEA.md`, its sole distinction
from `ask` (value sourced from the human) and `command` (value sourced from the
shell) is that the value must come from the **agent's own
reasoning/investigation** — and that "source of value" cue is supposed to live
in the node template.

But the `collect` template rendered only the instruction text plus a
conditional report-back line. It never stated the `collect` semantic, so the
model received no signal that it must actually perform the investigative /
analytical work the instruction describes and shape the reported value as the
product of that reasoning. The rendered prompt was indistinguishable from a
bare instruction + "set a variable", and was thinner even than the already
upgraded `ask` template (which carries a `<method>` block).

Observed live: a `collect` step rendered as `<instruction>…</instruction>`
followed by `<report>…</report>` with zero `collect`-semantics guidance.

## Root cause

Failing path: the `collect` node template body — both the shipped file
`.steer/templates/default/collect.j2.md` and the hardcoded fallback
`COLLECT_BODY` in `crates/steer-core/src/template.rs`. Neither conveyed the
"reason yourself" semantic; they only echoed the instruction and the report
mechanic.

## Fix

Add a `collect`-semantic directive (mirroring `ask`'s `<method>` block) that
tells the model: this is a reasoning op; the value must come from your own
investigation and analysis (not `ask` from the user, not `command` from a
shell); actually do the work the instruction describes; report the value that
work produces, grounded in evidence — do not guess.

- `.steer/templates/default/collect.j2.md` — added a `<method>` block between
  `<instruction>` and `<report>`.
- `crates/steer-core/src/template.rs` `COLLECT_BODY` — added the same semantic
  as a bullet, so the fallback (used when the file template is absent) is not
  separately broken.

The `<report>` line is unchanged (it already uses the self-contained
`steer instance set {{ instance }} {{ target }} <value>` form).

## Verification

- `value_nodes_differentiate_by_source` updated: `collect` now asserts the
  `Reasoning op` cue (and that it does not read like `ask`/`command`).
- New `collect_template_conveys_reasoning_semantic` — renders a `collect` call
  and asserts the reasoning semantic is present.
- New `collect_file_template_conveys_reasoning_semantic` — reads the shipped
  file template and asserts it carries the reasoning semantic, guarding the
  runtime path directly.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features` all
  pass.
- Live render from the repo root now shows the `<method>` block in the
  `collect` instruction.
