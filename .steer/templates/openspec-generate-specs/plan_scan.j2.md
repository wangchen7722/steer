---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the scan plan:
  <check>{{ check }}</check>
---
**plan-scan**: cluster the codebase into deep-scan units. Mining contract facts is expensive and burns context, so it pays to decide the boundaries up front from a shallow look, then commit to deep-mining each unit — rather than deep-mining blind and re-doing work when units turn out wrong. Read .openspec-generate-runs/{run}/env.md FIRST for the target-repo inventory, then do a SHALLOW functional clustering (structure and semantics, not contract facts) and record the resulting deep-scan units to scan-plan.md. Do not mine capabilities here — that is a later, separate task.

Fan out one shallow-cluster subagent per top-level source module/directory (e.g. `src/<area>`, a crate), across every target repo. Each clusters its module's code at the SURFACE only — what functional areas live here, how they group, where the natural deep-scan boundaries are — by skimming code structure and semantics, NOT by deep-reading git history, MRs, archived deltas, or contract facts (those belong to the deep-mining task, not this one). Each returns its module's cluster picture and the MAIN agent merges the per-module clusters into the final deep-scan units recorded to scan-plan.md.

Partition by code semantics, not by raw file count. The >10-file count is a signal that an area is large enough to merit its own deep-scan agent — it is NOT the partition key. Boundaries follow what the code does:

- A 50-file folder that is one cohesive mechanism → ONE unit (do not split it into N agents by file count).
- An 8-file folder spanning three distinct mechanisms → may be three units, or its mechanisms grouped with related code elsewhere.
- Size each unit for one deep-mine agent: not so big one agent can't mine it, not so small it's not worth an agent (a 3-file utility dir folds into a neighbor).

"Source files" means any file that carries behavior, not just `.rs`/`.c`/`.cpp`: include linker scripts (`.ld`/`.lds`), assembly (`.S`/`.s`/`.asm`), and any build artifact that defines behavior — a `SECTIONS` block, a `PROVIDE`/symbol definition, or an assembly routine is a functional unit in its own right and must land in some unit, never skipped as "non-source".

Every source file across every target repo must land in exactly one unit — no gaps, no double-counting. A cross-repo mechanism (code in repo A + repo B implementing one thing) is ONE unit spanning repos, not two — splitting it would cut one mechanism in half and lose the relationship between its sides. Note the cross-repo seam (which files/symbols in A call or depend on which in B) on the unit, so deep-mining can read both sides together. But specs stay per-repo: a cross-repo unit is mined as one, yet its contract facts are partitioned BY repo (A's side facts go to A's spec, B's to B's), and the cross-repo relationship is recorded as an inline link from one repo's spec to the other's, never merged into a single spanning spec.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Fill in the output template below, replacing each `<!-- ... -->` with real content and removing the placeholder.

<template>
# Scan Plan: <!-- run name -->

## Coverage check

- **Total source files across all target repos**: <!-- count -->
- **Total covered by the units below**: <!-- count — MUST equal the total above -->

## Scan units

For each deep-scan unit:

### Unit: <!-- kebab-case name -->

- **Repo(s)**: <!-- target repo path(s) this unit spans; one for single-repo, one or more for a cross-repo unit -->
- **Files/folders covered**: <!-- the source files/folders, relative to each repo's root -->
- **Cross-repo seam** (cross-repo units only): <!-- which files/symbols in repo A call or depend on which in repo B, and the cross-repo signal (Depends-On MR, close merge timestamps, doc/code cross-references); omit for single-repo units -->
- **Semantic summary**: <!-- one line: what this code area does -->
- **Why a unit**: <!-- the mechanism/coherence that makes this one deep-mine target; note if it was split from a large (>10-file) module or folded from small ones -->
- **Estimated complexity**: <!-- small / medium / large — roughly how many capabilities this unit's code looks likely to contain -->
</template>

<rules>
- LANGUAGE: write all output in English.
- Partition by code semantics; use the >10-file count only as a "this area is large" signal, never as the split key.
- Every source file in every target repo is covered by exactly one unit — no gaps, no double-counting.
- Do NOT mine capabilities or assign owners here — this step only plans WHERE to scan; mining and ownership come after, as separate tasks.
</rules>
