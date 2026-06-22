# steer examples

This directory contains realistic workflows that can be run from this directory.
The example templates live under `.steer/templates/` because the CLI resolves
templates relative to the current working directory.

Run from `/root/steer/examples`:

```bash
cargo run -q -p steer-cli --manifest-path ../Cargo.toml -- workflow validate workflows/bugfix-loop.steer
cargo run -q -p steer-cli --manifest-path ../Cargo.toml -- workflow simulate workflows/openspec-change.steer
```

Workflows:

- `workflows/bugfix-loop.steer` — root-cause-first bugfix with a bounded retry loop.
- `workflows/openspec-change.steer` — OpenSpec-style proposal -> design -> specs -> tasks flow.
- `workflows/template-switch.steer` — demonstrates `@template` switching with a planning template.
