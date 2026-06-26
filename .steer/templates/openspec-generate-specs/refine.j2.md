---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the refine round:
  <check>{{ check }}</check>
  Confirm coverage.md gained a `## Refine round {round}` block, every touched spec re-validates with `cd <repo> && openspec validate <cap> --type spec` exit 0 (requirements written to the behavior-owning repo, never the manifest root), and every behavior gap from the latest review round is now addressed by an added/merged behavior-level requirement or a new capability spec.
---
This is the openspec-generate-specs workflow. Current position: **refine** -- the acting half of the review<->refine closed loop. Close the per-repo behavior GAP lists from the latest review round by adding or merging behavior-level requirements INTO THE BEHAVIOR-OWNING REPO so each uncovered behavior contract is genuinely described. Do NOT create one requirement per helper, source file, build rule, script, or private implementation unit merely because it exists. DO add requirements for private/internal implementation when it embodies stable standard/protocol/bit-level behavior.

SPECS FOLLOW CODE AS EVIDENCE. Read the behavior GAP lists in coverage.md's latest `## Review round {round}` block (per repo). For each uncovered behavior contract, add coverage into the repo that owns that behavior (normally the repo whose code or contract-bearing artifact implements that side) -- that repo already has a `<cap>/spec.md` for any capability its code or artifacts participate in:

- PREFER adding a requirement (SHALL/MUST + scenario) describing the behavior contract to the most related EXISTING capability's spec in the behavior-owning repo (MERGE -- preserve existing content, only add). Most uncovered behavior belongs to a mechanism that is already specced but incompletely.
- ONE requirement may cover MULTIPLE evidence units that share one behavior -- do NOT create one requirement per evidence unit. Group gaps by the behavior they share and add a single requirement per shared behavior.
- Only if the behavior contract is a genuinely DISTINCT mechanism with no existing capability, create a NEW capability spec in the behavior-owning repo following the same OpenSpec format (SHALL/MUST, exactly-four-hashtag scenarios, Purpose >= 50 chars, behavior-contract standard). This is valid for low-level contract surfaces previously treated as implementation details -- linker memory layout (`link.x`, `.ld`, `.lds`), assembly vector table / startup entry ABI, device tree binding, ioctl/sysfs/netlink ABI, build-selected feature behavior (`BUILD.gn`, Blueprint, Make, product/board/generated config), SELinux/init/property policy, service registration, HAL manifest, compatibility matrix, or referenced protocol/standard obligations.
- A single new requirement can cover multiple gap evidence units if they share one mechanism -- do not create one requirement per evidence unit.
- NEVER write to the manifest root in a multi-repo checkout -- write into the behavior-owning repo's `openspec/specs/`. Do NOT write a behavior's requirement into a different repo; specs follow code as evidence.
- If the behavior belongs to a cross-repo capability whose primary-owner is a DIFFERENT repo, the behavior-owning repo already has (or now gets) its own local-side `<cap>/spec.md`; add the requirement there, not in the owner repo.

After editing, re-validate EVERY touched repo: `cd <repo> && openspec validate <cap> --type spec` until each exits 0.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file. APPEND this block to coverage.md (do not overwrite prior rounds).

<template>
## Refine round <!-- round -->

- **Behavior gaps addressed**: <!-- N behavior contracts, across all repos -->
- **Specs edited (MERGE)**: <!-- list each: <repo>/openspec/specs/<cap>/spec.md -- requirements/scenarios added -->
- **New capability specs created**: <!-- list each new <repo>/openspec/specs/<cap>/spec.md (common for linker/asm units with no prior cap), or "none" -->

### Coverage closure

For each behavior gap from the latest review round:

- `<!-- behavior contract summary -->` (owned by `<!-- repo -->`, evidenced by `<!-- file path / symbol / artifact -->`) -> now covered by `<!-- <repo>/openspec/specs/<cap>/spec.md, Requirement: <name> -->` (<!-- added new requirement / merged into existing -->)

### Re-validation

- `<!-- repo -->`: `cd <repo> && openspec validate <cap> --type spec` -> <!-- exit 0 / fixed ERROR then exit 0 -->
- **All touched specs validate?**: <!-- YES / NO -->
- **Anything written to the manifest root?**: <!-- NO (must always be NO in a multi-repo checkout) -->
- **Any line numbers written into spec.md?**: <!-- NO (must always be NO; file/symbol TRACE only) -->
</template>

<rules>
- LANGUAGE: Write all output in English. Code/identifiers follow existing project conventions.
- Write each refinement into the behavior-owning repo's `openspec/specs/` (normally the repo whose code or contract-bearing artifact implements that side -- specs follow code as evidence). In a multi-repo checkout NEVER write to the manifest root (it is not a git repo; untracked, wiped by `repo sync`), and NEVER write a behavior's requirement into a different repo.
- Requirement text MUST contain SHALL or MUST (no should/may). Every requirement MUST have at least one `#### Scenario:` block using EXACTLY four hashtags. Purpose >= 50 chars for any NEW capability spec. Every new/edited scenario MUST end with a `- **TRACE**:` field at stable granularity (module / source file / function-symbol, coarsest sufficient) -- never a line number or `file:line` range. Scenario WHEN/THEN steps are black-box (externally observable event/consequence); private symbols go in the TRACE at most, never in the steps or the SHALL/MUST sentence; only contract-surface identifiers appear in steps.
- One requirement may cover MULTIPLE evidence units sharing one behavior -- do not spec one requirement per unit. Group gaps by shared behavior.
- Pin concrete identifiers, numeric values, timeouts, signatures, trait boundaries, state transitions, linker/asm section names, addresses, alignment, symbol names, vector offsets, build/config names, policy names, protocol IDs only when they are part of the external contract or compatibility promise -- same standard as generate-all. Otherwise record them as evidence (in the scenario TRACE, never in requirement prose). Line numbers and line ranges MUST NOT appear anywhere in spec text. "SHALL handle appropriately" is a defect.
- Create a NEW capability only for a genuinely distinct mechanism with no existing capability. Low-level contract surfaces (linker-script layouts, assembly routines, device tree bindings, build-selected feature behavior, policy files, manifests, compatibility matrices, protocol tables) that no existing cap describes are legitimate triggers for a new capability -- do not leave them unspecced as "implementation details".
- Reuse existing capability names; do not rename, merge, or drop one. Add NEW capabilities only for genuinely distinct mechanisms.
- Re-validate every touched repo (`cd <repo> && openspec validate <cap> --type spec`) until exit 0. Do not claim closure for a spec that still fails validation.
- APPEND this block to coverage.md; never overwrite prior rounds.
- Do NOT re-extract for the gap here -- that is the next review round's job. This step only closes the known gap and re-validates.
- Execute only this instruction.
</rules>
