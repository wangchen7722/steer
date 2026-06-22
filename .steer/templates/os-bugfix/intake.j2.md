---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the intake record:
  <check>{{ check }}</check>
  Confirm every metadata field is filled with a concrete value.
---
This is an OS-domain bugfix workflow (Rust + GN/Ninja, manifest-scale multi-repo, real-device flash). Current position: **intake** — capture bug metadata so every later phase and every resumed run shares the same context.

Interview the user (via AskUserQuestion where a field is unclear) and write the intake record. This file is the durable header of the dossier; one file per bug.

The dossier (`bugfix/<slug>/`) collects, across iterations: intake.md, reproduce.md, rca.md, findings.md (append-only), iterations.md (append-only), fix.md, flash.md, verify.md, and handoff.md (only on failure).

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
# Bug Intake: <!-- short title -->

- **Slug**: <!-- kebab-case slug, matches the dossier directory name -->
- **Component / crate**: <!-- subsystem, crate, or git project this belongs to -->
- **Severity**: <!-- blocker / critical / major / minor -->
- **OS / build version**: <!-- image or build id, branch, build type -->
- **Target device / board**: <!-- device, board, or product name -->
- **Environment**: <!-- hardware rev, peripherals, network, anything relevant -->
- **Reporter / date**: <!-- who reported it and when -->

## Symptom

<!-- One-paragraph description of the observed malfunction. -->

## Status

<!-- OPEN -->
</template>

<rules>
- LANGUAGE: Write all output in English. Code and identifiers follow existing project conventions.
- Every field above must have a concrete value; no placeholder left unfilled.
- Status starts as OPEN.
- Execute only this instruction. Do NOT skip ahead or do unplanned work.
</rules>
