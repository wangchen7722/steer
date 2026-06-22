---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the fix:
  <check>{{ check }}</check>
  Confirm the change is applied and fix.md records changed files, touched projects, and rationale.
---
Current position: **develop fix**. Apply the smallest safe change that addresses the root cause, not the symptom.

FIRST read the current bug's rca.md (the root cause) AND iterations.md and findings.md (prior attempts). Your fix for this attempt MUST differ from prior failed attempts — do not repeat a fix that already failed.

This is Rust + GN/Ninja: edit the source in the affected git project(s); keep the change minimal and idiomatic to the surrounding code; do not reformat unrelated code.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Rewrite fix.md for this attempt using the template below (each attempt overwrites fix.md with the current attempt's change record).

<template>
# Fix: <!-- slug --> — attempt <!-- N -->

## Change summary

<!-- one-paragraph description of what was changed and why it addresses the root cause -->

## Changed files

- <!-- repo-project / path — what changed -->
- <!-- ... -->

## Touched git project(s)

- <!-- project name(s), must match the affected projects in rca.md -->

## Why this should work

<!-- tie directly to the causal chain in rca.md; state which cause it removes -->

## How it differs from prior attempts

<!-- if attempt > 1, explicitly state how this differs from earlier failed fixes; otherwise write 'first attempt' -->
</template>

<rules>
- LANGUAGE: English prose; code follows project conventions.
- Smallest safe change only; preserve all other behavior.
- fix.md must list every file you changed and every project you touched.
- Do NOT build or flash here; later steps do that.
- Execute only this instruction.
</rules>
