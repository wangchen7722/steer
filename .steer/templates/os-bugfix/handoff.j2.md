---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the handoff:
  <check>{{ check }}</check>
  Confirm the handoff states root cause, every attempted fix, last evidence, and the next step.
---
Current position: **handoff**. The bug is not fixed after the attempt budget was exhausted. Write a handoff so the next engineer (or the next run) can continue without re-deriving everything. Read the full dossier first: rca.md, iterations.md, findings.md, fix.md, verify.md.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Write handoff.md using this template.

<template>
# Handoff: <!-- slug --> — unfixed after <!-- N --> attempts

## Bug summary

<!-- one-paragraph restatement of the symptom -->

## Root cause (best current understanding)

<!-- from rca.md; note confidence -->

## Attempts

<!-- for each attempt: what was tried, what happened, why it failed -->

## Last failing evidence

<!-- the most recent verify.md result + evidence pointer -->

## Next diagnostic step

<!-- the single most promising next action to move this forward -->

## Open questions / hypotheses

<!-- unresolved threads worth pursuing -->
</template>

<rules>
- LANGUAGE: English output.
- Be specific and honest; a vague handoff wastes the next run.
- Read the dossier; do not contradict recorded evidence.
- Execute only this instruction.
</rules>
