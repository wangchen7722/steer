# steer examples

This directory contains example workflows and templates that demonstrate steer's
language features and the OpenSpec + Superpowers integration.

Run from the repo root:

```bash
steer workflow validate examples/workflows/openspec-propose.steer
steer workflow simulate examples/workflows/openspec-propose.steer
```

Workflows:

- `openspec-propose.steer` — OpenSpec propose phase: brainstorm, proposal, specs, design, tasks, plan. Pauses for human review.
- `openspec-apply.steer` — OpenSpec apply phase: execute the plan, then verify. Requires the propose workflow to have run first.

Templates:

- `templates/openspec-superpowers/` — Custom callee templates for the OpenSpec phases (`brainstorm`, `proposal`, `specs`, `design`, `tasks`, `plan`, `apply_change`, `verify`). The workflows activate this directory via `@template = "openspec-superpowers"`.
