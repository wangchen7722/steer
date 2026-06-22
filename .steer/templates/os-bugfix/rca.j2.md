---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the root-cause analysis:
  <check>{{ check }}</check>
  Confirm the RCA names affected git project(s), the failing code location, the causal chain, and the evidence.
---
Current position: **root-cause analysis (RCA)**. Find WHY the bug happens and WHERE, at the level of specific code. This codebase is manifest-scale (many git repositories): you MUST identify which git project(s) the failing code lives in.

FIRST read the current bug's reproduce.md (the evidence) and findings.md / iterations.md if present.

Method (do real work, ground every claim):
- Trace from the captured logs, panic, or backtrace to the failing crate, module, function, and source line.
- Identify the affected git project(s): use the manifest/repo tooling or `git remote -v` / `repo manifest` to map source paths to project names.
- Use `git log` / `git blame` to find recent relevant changes if useful.
- State observed vs expected divergence precisely.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template for rca.md. Append a dated entry to findings.md.

<template>
# Root-Cause Analysis: <!-- slug -->

## Affected git project(s)

- <!-- project name + repo path, identified via manifest or git remote -->
- <!-- ... -->

## Failing location

- **Crate / module**: <!-- ... -->
- **File : line range**: <!-- path:range -->
- **Function / symbol**: <!-- ... -->

## Causal chain

1. <!-- cause step -->
2. <!-- cause step -->
3. <!-- ... leads to the observed symptom -->

## Observed vs expected

- **Observed**: <!-- ... -->
- **Expected**: <!-- ... -->
- **Why they diverge**: <!-- the actual defect: race, off-by-one, null deref, wrong config, etc. -->

## Evidence

- <!-- log / trace / blame pointers from reproduce.md or new captures -->

## Confidence

<!-- High / Medium / Low — and what would raise or lower it -->
</template>

For findings.md, append a block shaped like:

<template>
## <!-- YYYY-MM-DD HH:mm --> — RCA

- <!-- one-line root-cause finding -->
- <!-- affected project pointer -->
</template>

<rules>
- LANGUAGE: English output.
- The affected git project(s) MUST be identified, not just the file path.
- Every causal claim must point to evidence; no speculation presented as fact.
- Append to findings.md only.
- Execute only this instruction.
</rules>
