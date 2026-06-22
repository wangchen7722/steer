---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the reproduction:
  <check>{{ check }}</check>
  Confirm the repro is concrete, repeatable, and backed by captured evidence.
---
Current position: **reproduce** — establish a reliable, repeatable reproduction and capture hard evidence (logs, traces). A bug you cannot reproduce, you cannot verify as fixed.

If a dossier already exists, FIRST read the current bug's intake.md (and findings.md, iterations.md if present) so you build on prior context instead of redoing work.

Do real work: run the system, trigger the symptom, and capture logs. For a Rust/GN+Ninja OS codebase, relevant evidence typically includes: kernel/system logs, Rust panic backtraces (RUST_BACKTRACE), service or crash logs, tombstone-like crash records, and the exact build id on the device. Save captured logs under `bugfix/<slug>/logs/` and reference them by relative path.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template for reproduce.md. Append (do not overwrite) a new entry to findings.md.

<template>
# Reproduction: <!-- slug -->

## Steps to reproduce

1. <!-- concrete step -->
2. <!-- concrete step -->
3. <!-- concrete step -->

## Observed behavior

<!-- What actually happens, including any crash, error, or panic text. -->

## Expected behavior

<!-- What should happen instead. -->

## Environment at reproduction

- **Build id**: <!-- read from the device -->
- **Device / board**: <!-- ... -->
- **How triggered**: <!-- app, test, boot, stress, etc. -->

## Captured evidence

- <!-- relative path or inline snippet, e.g. logs/boot-panic.log -->
- <!-- ... -->

## Reproducibility

<!-- Always / Intermittent (rough rate) / Once. Note any precondition. -->
</template>

For findings.md, append a block shaped like:

<template>
## <!-- YYYY-MM-DD HH:mm --> — reproduce

- <!-- one-line finding, e.g. panic originates in crate X, function Y -->
- <!-- evidence pointer -->
</template>

<rules>
- LANGUAGE: English output.
- Repro steps must be concrete and repeatable by another engineer.
- Do not guess the symptom; ground every claim in captured evidence.
- Append to findings.md only; never rewrite prior entries.
- Execute only this instruction.
</rules>
