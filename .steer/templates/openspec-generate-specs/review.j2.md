---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the review round:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **review** -- the sensing half of the review<->refine closed loop. Go through every meaningful piece of behavior in the code: file by file, for every FUNCTIONAL UNIT, check whether a spec requirement genuinely describes it. A unit with a covering requirement PASSES (and is recorded, so later rounds skip it); a unit with none FAILS into the per-repo GAP list. The GAP lists you produce are the input to the refine step.

COVERAGE SCOPE -- functional units from EVERY file, not just public source functions:
- A FUNCTIONAL UNIT is a named unit of behavior. Examples: a function of ANY visibility (`pub fn`, `pub(crate) fn`, or private `fn`); a trait method; an assembly label/routine (`.S`/`.s`/`.asm`); a linker-script SECTIONS sub-block, `PROVIDE(...)` or symbol definition (`.ld`/`.lds`); any build artifact that carries behavior.
- Extract units from EVERY file, not only `.rs`/`.c`/`.h` source: include linker scripts and assembly. A unit's repo is the repo its FILE lives in (in a single-repo project that is the project root).
- EXCLUDE only TRIVIAL units with no observable behavior transform: pure getter/setter passthrough, a single constant assignment, pure formatting wrappers. They have no independent behavior to spec -- their effect is that of the requirement they participate in -- so they are NOT enumerated separately. This is the ONLY exclusion; do not exclude private/internal units for being non-public.

A functional unit counts as COVERED only if a REQUIREMENT in its own repo's `openspec/specs/` genuinely DESCRIBES that unit's observable behavior -- a `### Requirement:` whose SHALL/MUST + scenario(s) reflect what the unit does. A passing mention of a file path in an unrelated spec does NOT count; cite the SPECIFIC requirement (`<cap>#<requirement-name>`) that covers the unit, not just the spec file.

INCREMENTAL RESCAN VIA THE PASSED-UNITS LOG. Re-checking every unit from scratch every round is wasteful: the codebase is static during the run, and most units, once covered, stay covered. So a unit recorded PASS in an earlier round (with its file path + covering requirement, in this shard's passed-units log) is SKIPPED on later rounds -- the subagent trusts the prior PASS and does not re-read the code for it. Each round a subagent only re-checks:
- the units that were GAP in the latest review round (refine was supposed to cover them -- verify it did), AND
- any unit whose covering spec a refine round EDITED since it was marked PASS (refine may have reworded or removed its covering requirement -- a PASS recorded against a now-changed requirement is no longer trustworthy, so re-check it).

Everything else stays skipped. Newly-passed units get appended to this shard's passed-units log. This is incremental on purpose -- but the two re-check triggers above are what keep it honest: GAP units always come back under scrutiny, and refine's own edits invalidate the PASS records they touch.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Fan out one subagent per module shard (multi-repo: per repo; single-repo: per top-level module). Each subagent:
1. Reads its shard's passed-units log under `.openspec-generate-runs/{run}/coverage-passed/<shard-slug>.md` (if it exists from a prior round) -- those units are PASS and skipped (subject to the refine-edited rule above).
2. Extracts the remaining functional units in its shard's files, and for each, finds the covering requirement in that unit's own repo's specs (or NONE).
3. Appends every newly-passed unit to its shard's passed-units log (file path + covering requirement) -- one shard log, appended across rounds, never overwritten.
4. Returns its shard's GAP list (units with no covering requirement) to the main agent.
The main agent merges the shard GAP lists into one `## Review round {round}` block, APPENDED to coverage.md (never overwrite prior rounds); also append a copy to review.md. Two subagents never touch the same shard log (one subagent per shard), so concurrent appends are safe.

Use the following as your output templates. Follow the structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments.

<template>
## Review round <!-- round -->

Per-repo GAP lists, merged from every shard's subagent. (Passed units live in the per-shard passed-units logs under coverage-passed/, not here -- this block is only the still-uncovered units.)

### Repo: <!-- repo path (or "project root") -->

- **Shards reviewed**: <!-- list the shard slugs under this repo that had a subagent this round -->
- **Units skipped (already PASS in this shard's log)**: <!-- N, from the passed-units log; M of those re-checked because refine edited their covering spec -->
- **Units re-checked this round**: <!-- N -- the prior-round GAP units + refine-edited PASS units -->

#### GAP list (uncovered functional units in this repo)

- `<!-- unit kind: function / trait method / asm routine / linker SECTIONS block / linker symbol -->` `<!-- unit name or location -->` (`<!-- file path:line -->`) -> NONE -- <!-- why no covering requirement: no requirement describes its behavior; cite what is missing -->
- <!-- "none -- every functional unit in this repo is covered by a requirement" if the gap is empty -->

### Aggregate verdict

- <!-- COVERED (every target repo's gap list is empty) or UNCOVERED (N functional units still lack a covering requirement, across these repos: <list>) -->
</template>

Per-shard passed-units log, at `.openspec-generate-runs/{run}/coverage-passed/<shard-slug>.md` (multi-repo: `<repo-slug>.md`; single-repo: `<module-slug>.md`). Append to it across rounds; never overwrite. Each subagent writes ONLY its own shard's log.

<template>
# Passed Units -- <!-- shard-slug (repo path or module) -->

Appended across review rounds. A unit listed here is PASS -- later rounds skip it unless a refine round edited its covering spec (then it is re-checked and, if still covered, re-recorded with the new requirement).

## Round <!-- round -->

For each functional unit newly confirmed PASS this round:

- `<!-- unit kind -->` `<!-- unit name or location -->` (`<!-- file path:line -->`) -> `<!-- covering requirement <cap>#<requirement-name> -->`
</template>

<rules>
- LANGUAGE: English output.
- Do NOT fix the gap here -- that is the refine step. This step only senses, records PASS, and reports GAP.
</rules>
