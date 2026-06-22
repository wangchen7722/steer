---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the iteration log:
  <check>{{ check }}</check>
  Confirm iterations.md gained a dated attempt entry and findings.md gained any new finding.
---
Current position: **log iteration**. Record what this attempt did and how it turned out, so the next iteration (or a resumed run) does not repeat a failed fix. Read the current attempt's fix.md and verify.md to summarize accurately.

Append a dated entry to iterations.md, and append any NEW finding from this attempt to findings.md (skip the findings append if nothing new was learned).

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Append to iterations.md using this template (one block per attempt, newest last).

<template>
## Attempt <!-- N --> — <!-- YYYY-MM-DD HH:mm -->

- **Fix applied**: <!-- one-line summary + changed project(s) -->
- **Build**: <!-- OK | FAILED (error) -->
- **Flash**: <!-- OK | FAILED (reason) | SKIPPED -->
- **Verify verdict**: <!-- FIXED | NOT-FIXED | INCONCLUSIVE -->
- **Outcome**: <!-- what this attempt proved or disproved -->
</template>

If this attempt produced a new insight, append to findings.md:

<template>
## <!-- YYYY-MM-DD HH:mm --> — attempt <!-- N --> finding

- <!-- new thing learned, e.g. 'fix A did not address the race in X' -->
</template>

<rules>
- LANGUAGE: English output.
- iterations.md is append-only; never edit or remove prior attempt blocks.
- findings.md append-only; only add an entry if this attempt produced a genuinely new insight.
- Execute only this instruction.
</rules>
