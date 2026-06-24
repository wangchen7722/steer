---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the refine round:
  <check>{{ check }}</check>
  Confirm coverage.md gained a `## Refine round {round}` block, every touched spec re-validates with `cd <repo> && openspec validate <cap> --type spec` exit 0 (requirements written to the functional unit's own repo, never the manifest root), and every gap functional unit from the latest review round is now addressed by an added/merged requirement or a new capability spec.
---
This is the openspec-generate-specs workflow. Current position: **refine** -- the acting half of the review<->refine closed loop. Close the per-repo GAP lists from the latest review round by adding or merging requirements INTO THE UNIT'S OWN REPO so each uncovered FUNCTIONAL UNIT gets a requirement that genuinely describes its behavior.

SPECS FOLLOW CODE. Read the GAP lists in coverage.md's latest `## Review round {round}` block (per repo). For each uncovered functional unit, add coverage into ITS OWN repo (the repo the unit's FILE lives in) -- that repo already has a `<cap>/spec.md` for any capability its code participates in:

- PREFER adding a requirement (SHALL/MUST + scenario) describing that unit's behavior to the most related EXISTING capability's spec in that unit's repo (MERGE -- preserve existing content, only add). Most uncovered units belong to a mechanism that is already specced but incompletely.
- ONE requirement may cover MULTIPLE functional units that share one behavior -- do NOT create one requirement per unit. Group gap units by the behavior they share and add a single requirement per shared behavior.
- Only if the unit implements a genuinely DISTINCT mechanism with no existing capability, create a NEW capability spec in that unit's repo following the same OpenSpec format (SHALL/MUST, exactly-four-hashtag scenarios, Purpose >= 50 chars, pinned contract facts). This is COMMON for non-source units previously treated as implementation details -- a linker-script SECTIONS layout, an assembly vector table / startup routine -- that no existing capability describes; give them their own capability (e.g. `boot-layout`, `exception-vectors`) in the repo the file lives in. Reuse existing capability names; do not rename or drop.
- A single new requirement can cover multiple gap units if they share one mechanism -- do not create one requirement per unit.
- NEVER write to the manifest root in a multi-repo checkout -- write into the unit's own repo's `openspec/specs/`, the repo the unit's FILE lives in. Do NOT write a unit's requirement into a different repo; specs follow code.
- If the unit belongs to a cross-repo capability whose primary-owner is a DIFFERENT repo, the unit's repo already has (or now gets) its own local-side `<cap>/spec.md`; add the requirement there, not in the owner repo.

After editing, re-validate EVERY touched repo: `cd <repo> && openspec validate <cap> --type spec` until each exits 0.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file. APPEND this block to coverage.md (do not overwrite prior rounds).

<template>
## Refine round <!-- round -->

- **Gap functional units addressed**: <!-- N, across all repos -->
- **Specs edited (MERGE)**: <!-- list each: <repo>/openspec/specs/<cap>/spec.md -- requirements/scenarios added -->
- **New capability specs created**: <!-- list each new <repo>/openspec/specs/<cap>/spec.md (common for linker/asm units with no prior cap), or "none" -->

### Coverage closure

For each gap functional unit from the latest review round:

- `<!-- unit -->` (in `<!-- repo -->`, `<!-- file path -->`) -> now covered by `<!-- <repo>/openspec/specs/<cap>/spec.md, Requirement: <name> -->` (<!-- added new requirement / merged into existing -->)

### Re-validation

- `<!-- repo -->`: `cd <repo> && openspec validate <cap> --type spec` -> <!-- exit 0 / fixed ERROR then exit 0 -->
- **All touched specs validate?**: <!-- YES / NO -->
- **Anything written to the manifest root?**: <!-- NO (must always be NO in a multi-repo checkout) -->
</template>

<rules>
- LANGUAGE: Write all output in English. Code/identifiers follow existing project conventions.
- Write each refinement into the uncovered unit's OWN repo's `openspec/specs/` (the repo the unit's FILE lives in -- specs follow code). In a multi-repo checkout NEVER write to the manifest root (it is not a git repo; untracked, wiped by `repo sync`), and NEVER write a unit's requirement into a different repo. A linker-script or assembly unit is written to the repo that file lives in, just like a source unit.
- Requirement text MUST contain SHALL or MUST (no should/may). Every requirement MUST have at least one `#### Scenario:` block using EXACTLY four hashtags. Purpose >= 50 chars for any NEW capability spec.
- One requirement may cover MULTIPLE functional units sharing one behavior -- do not spec one requirement per unit. Group gap units by shared behavior.
- Pin concrete contract facts (identifiers + numeric values, timeouts, signatures, trait boundaries, state transitions, and for linker/asm units: section names, addresses, alignment, symbol names, vector offsets) -- same standard as generate-all. "SHALL handle appropriately" is a defect.
- Create a NEW capability only for a genuinely distinct mechanism with no existing capability. Non-source units (linker-script layouts, assembly routines) that no existing cap describes are a legitimate trigger for a new capability -- do not leave them unspecced as "implementation details".
- Reuse existing capability names; do not rename, merge, or drop one. Add NEW capabilities only for genuinely distinct mechanisms.
- Re-validate every touched repo (`cd <repo> && openspec validate <cap> --type spec`) until exit 0. Do not claim closure for a spec that still fails validation.
- APPEND this block to coverage.md; never overwrite prior rounds.
- Do NOT re-extract for the gap here -- that is the next review round's job. This step only closes the known gap and re-validates.
- Execute only this instruction.
</rules>
