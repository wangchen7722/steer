---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the verification record:
  <check>{{ check }}</check>
  Confirm verify.md records this attempt's result and evidence.
---
Current position: **verify**. Determine whether the bug still reproduces after this attempt's flash. Read the verify command from the current bug's verify.md.

- If the verify command is host-driven, run it and capture output.
- If verification must happen on the device (host cannot drive it), use AskUserQuestion to ask the human to run the reproduction on the device and report: does the bug still occur? Ask them to attach the relevant logs.
- Append this attempt's result and evidence to verify.md (do not overwrite earlier attempts' blocks).

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Append an entry to verify.md using this template (one block per attempt, newest last).

<template>
## Attempt <!-- N --> — <!-- PASS | FAIL | INCONCLUSIVE --> — <!-- YYYY-MM-DD HH:mm -->

- **Verify command run**: <!-- the command, or 'human-on-device' -->
- **Result**: <!-- bug still reproduces? yes/no, with the observed behavior -->
- **Evidence**: <!-- log path or inline snippet -->
- **Verdict**: <!-- FIXED (no longer reproduces) | NOT-FIXED | INCONCLUSIVE -->
</template>

<rules>
- LANGUAGE: English output.
- Ground the verdict in evidence, not optimism.
- Append only; keep prior attempts' blocks intact.
- Execute only this instruction.
</rules>
