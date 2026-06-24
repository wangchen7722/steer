---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the coverage guard:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **coverage-guard** -- the gate that `openspec validate` CANNOT provide. Validate checks structure only; it happily passes a spec that silently dropped half the project's existing requirements. This gate runs ONLY for target repos where prior `openspec/specs/` existed (the workflow already branched on env.md before calling this step).

Fan out one subagent PER such target repo -- the per-repo check is independent (each repo diffs its own existing-vs-generated specs), so run them in parallel; within a repo the subagent works serially (it may edit that repo's specs to fix regressions, and two subagents never touch the same repo). A capability name may now legitimately exist in multiple repos (specs follow code: each participating repo has its own `<cap>/spec.md`); each repo is checked INDEPENDENTLY against ITS OWN prior floor -- do not conflate repos. For EVERY capability name recorded as existing in that repo (per env.md):
1. Confirm a generated spec exists under the SAME directory name in THAT repo's `openspec/specs/`. Missing or renamed is a FAIL.
2. Diff requirement-by-requirement: each existing `### Requirement:` (and its scenarios) must be present in the regenerated spec in that repo, preserving the contract facts (IDs, values, timeouts, signatures).
3. Any existing requirement/scenario/contract fact absent from the generated spec is a REGRESSION -- add it to that repo's spec, then re-run `cd <repo> && openspec validate <cap> --type spec` until exit 0.

Each subagent RETURNS its repo's `### Repo:` block (per the template below) to the main agent; the main agent APPENDS the returned repo blocks plus the `### Aggregate` block into one `## Coverage guard` block in coverage.md -- do NOT let subagents append to the shared coverage.md directly (concurrent appends lose blocks).

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file. APPEND this block to coverage.md (do not overwrite prior content).

<template>
## Coverage guard

Aggregate across all target repos that had prior specs.

### Repo: <!-- repo path (or "project root") -->

- **Existing capabilities checked**: <!-- N -->
- **All reproduced under same name in this repo?**: <!-- YES / NO -- if NO, list the missing/renamed one(s); this is a FAIL -->
- **Per-capability requirement diff**:
  - `<!-- cap -->`: <!-- M existing requirements reproduced; K net-new added; OR "REGRESSION: <requirement> missing -> fixed and re-validated via `cd <repo> && openspec validate`" -->

### Aggregate

- **Repos with prior specs**: <!-- list -->
- **Total existing capabilities reproduced**: <!-- N -->
- **Total existing requirements reproduced**: <!-- M -->
- **Total net-new requirements added**: <!-- K -->
- **Regressions found and fixed**: <!-- count, or "none" -->
- **Verdict**: <!-- PASS (every existing capability + requirement/scenario/contract fact reproduced across all repos, plus net-new additions) or FAIL (list unresolved regressions per repo) -->
</template>

<rules>
- LANGUAGE: English output.
- Run the guard PER target repo that had prior specs (per env.md). Do not conflate repos -- each repo's coverage floor is independent. A capability name may exist in multiple repos; check each repo only against its own prior floor.
- Use grep to list `### Requirement:` titles per capability and diff existing-vs-generated, INSIDE each repo. Example: `diff <(grep -E '^### Requirement:' <repo>/openspec/specs/<cap>/spec.md | sort) <(grep -E '^### Requirement:' <repo>/openspec/specs/<cap>/spec.md | sort)` (existing vs regenerated, same file after merge).
- Any existing requirement/scenario/contract fact absent from the generated spec is a regression. Fix it by ADDING to that repo's spec (preserve existing content), then re-run `cd <repo> && openspec validate <cap> --type spec` until exit 0.
- Do not report PASS with known regressions. Skipping this gate because "validate passed" is the exact failure mode this workflow exists to prevent.
- Execute only this instruction.
</rules>
