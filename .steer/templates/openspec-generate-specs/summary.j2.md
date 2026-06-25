---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the summary:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **summary** -- write the consolidated summary, the single handoff-ready artifact a reader can pick up without scanning the whole dossier. Read env.md, final-list.md, generation-log.md, coverage.md (including the coverage guard block and every review/refine round block), and review.md. The per-(capability, repo) table can run to hundreds or thousands of rows, so write it incrementally: write the file header and column legend first, then append rows in batches of ~20, then the totals/coverage-guard/review-outcome sections at the end — do not hold the whole table in memory and dump it once.

This is a report on what was generated, NOT more spec writing.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
# Spec Generation Summary: <!-- run name -->

## Specs generated

One row per (capability, repo) -- every participating repo of every capability is its own row.

| Capability | Repo | Owner? | Requirements | Scenarios | Status |
|------------|------|--------|-------------|-----------|--------|
| <!-- name --> | <!-- repo path (project root for single-repo; participating sub-repo for multi-repo) --> | <!-- Y (primary-owner) / N (participant) --> | <!-- N --> | <!-- M --> | <!-- New / Merged --> |

Total: <!-- X --> capabilities, <!-- Y --> (capability, repo) spec files, <!-- Z --> requirements, <!-- W --> scenarios

## Coverage guard

<!-- Per repo: "repo R: all N existing capabilities and M existing behavior requirements reproduced; K net-new behavior requirements added." OR "No prior specs -- bootstrap." -->

## Review<->refine loop

- **Rounds run**: <!-- round -->
- **Outcome**: <!-- COVERED -- the last review round's GAP was already empty: every meaningful behavior contract discovered from scanned evidence in every repo (source, config, build, script, IDL, policy, manifest, linker/assembly, and protocol/standard evidence; excluding only trivial no-transform evidence) is described by a requirement in the behavior-owning repo. OR UNCOVERED -- the last review round still had behavior GAPs; refine ran after it but was NOT re-reviewed to confirm, so the gaps below (from the last review round) may or may not have been closed by refine -- the user should verify. -->
- **Residual gap** (if uncovered): per repo, list each behavior contract the LAST REVIEW round found without a covering requirement, with evidence units for traceability (refine may have since addressed some -- verify against the refine round blocks in coverage.md):
  - `<!-- repo -->`: `<!-- behavior contract -->` evidenced by `<!-- file path / symbol / artifact -->`

## Files

Final results:
- Specs: written to each repo's own `openspec/specs/<cap>/spec.md` (one per participating repo of each capability; related specs are linked inline in requirement descriptions where context helps). NEVER written to the manifest root.
- This summary: `.openspec-generate-runs/{run}/summary.md`

Audit dossier (intermediate artifacts, all under `.openspec-generate-runs/{run}/`):
- `env.md` -- target-repo inventory: single-repo vs multi-repo, each repo's path, root status, and whether it had prior `openspec/specs/`.
- `scan-plan.md` -- the deep-scan plan: which files/modules/artifacts to scan per repo for behavior evidence.
- `capabilities/` -- gathered context per capability, sharded per repo/module (the gather step's output).
- `final-list.md` -- the FINAL capability list: each capability with its primary-owner repo, participating-repos list, per-repo REUSE-vs-NEW, behavior contracts to preserve, and implementation evidence.
- `generation-log.md` -- every (capability, repo) spec written this run, with its role, spec path, inline links emitted, requirement/scenario counts, New/Merged status, and per-(cap,repo) validate result.
- `coverage.md` -- the coverage-guard block plus every `## Review round N` and `## Refine round N` block appended across the loop.
- `review.md` -- copies of every review round block (same content as the review blocks in coverage.md).
- `coverage-passed/` -- per-shard logs (one `<shard-slug>.md` per repo or top-level module) of behavior/evidence pairs already recorded PASS, the skip-set for incremental rescans.
</template>

<rules>
- LANGUAGE: English output.
- The Repo column is MANDATORY -- it lets the user verify each repo's code got its own local spec and that nothing was written to the manifest root.
- Be honest: report covered only when the dossier genuinely shows every meaningful behavior contract discovered from scanned evidence is covered, and do not invent findings not present in the dossier files.
</rules>
