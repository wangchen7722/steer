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
This is the openspec-generate-specs workflow. Current position: **review** -- the sensing half of the review<->refine closed loop. Scan every behavior-bearing evidence unit in the codebase and determine whether the meaningful behavior contracts they reveal are covered by spec requirements. A behavior contract with a covering requirement PASSES (and its supporting evidence units are recorded, so later rounds skip them); an uncovered behavior contract FAILS into the per-repo GAP list. A private helper, local call sequence, build rule, or script is NOT a gap merely because it lacks its own requirement -- it is evidence for a gap only when it reveals an uncovered observable rule, boundary, error semantic, invariant, compatibility surface, ABI, protocol, policy, layout, lifecycle, state transition, industry-standard parsing semantic, or bit/field meaning.

COVERAGE SCOPE -- behavior contracts discovered from evidence units in EVERY file, not just public source functions:
- An EVIDENCE UNIT is a named or locatable unit that may reveal behavior. Examples: a function of ANY visibility (`pub fn`, `pub(crate) fn`, or private `fn`); a trait method; an assembly label/routine (`.S`/`.s`/`.asm`); a linker-script SECTIONS sub-block, `PROVIDE(...)` or symbol definition (`link.x`, `.ld`, `.lds`); Kconfig/defconfig/device-tree entries; `BUILD.gn`, `Android.bp`, `Android.mk`, GN args, product/board config, generated config headers; init rc/system properties; SELinux policy and permission declarations; AIDL/HIDL/Binder interfaces; HAL manifests and compatibility matrices; Python/shell/generator scripts that define generated APIs, protocol tables, packaging, or compatibility checks; external standards/protocol references cited by code/tests/docs.
- Extract evidence units from EVERY behavior-bearing file. The behavior-owning repo is the repo whose code or contract-bearing artifact implements that side of the behavior (in a single-repo project that is the project root).
- EXCLUDE only TRIVIAL evidence with no observable behavior transform: pure getter/setter passthrough, a single constant assignment, pure formatting wrappers. They have no independent behavior to spec -- their effect is that of the requirement they participate in -- so they are NOT enumerated separately. This is the ONLY exclusion; do not exclude private/internal evidence for being non-public.

A behavior contract counts as COVERED only if a REQUIREMENT in the behavior-owning repo's `openspec/specs/` genuinely DESCRIBES that behavior -- a `### Requirement:` whose SHALL/MUST + scenario(s) reflect the observable rule, boundary, error semantic, invariant, compatibility surface, ABI, protocol, policy, layout, lifecycle, or state transition. A passing mention of a file path in an unrelated spec does NOT count; cite the SPECIFIC requirement (`<cap>#<requirement-name>`) that covers the behavior, not just the spec file.

INCREMENTAL RESCAN VIA THE PASSED-UNITS LOG. Re-checking every evidence unit from scratch every round is wasteful: the codebase is static during the run, and most behavior, once covered, stays covered. So evidence recorded PASS in an earlier round (with its file path + covering requirement, in this shard's passed-units log) is SKIPPED on later rounds -- the subagent trusts the prior PASS and does not re-read the evidence for it. Each round a subagent only re-checks:
- the behavior gaps from the latest review round (refine was supposed to cover them -- verify it did), AND
- any evidence whose covering spec a refine round EDITED since it was marked PASS (refine may have reworded or removed its covering requirement -- a PASS recorded against a now-changed requirement is no longer trustworthy, so re-check it).

Everything else stays skipped. Newly-passed units get appended to this shard's passed-units log. This is incremental on purpose -- but the two re-check triggers above are what keep it honest: GAP units always come back under scrutiny, and refine's own edits invalidate the PASS records they touch.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Fan out one subagent per module shard (multi-repo: per repo; single-repo: per top-level module). Each subagent:
1. Reads its shard's passed-units log under `.openspec-generate-runs/{run}/coverage-passed/<shard-slug>.md` (if it exists from a prior round) -- that evidence is PASS and skipped (subject to the refine-edited rule above).
2. Extracts the remaining behavior-bearing evidence units in its shard's files, groups them by the behavior contract they support, and for each behavior contract, finds the covering requirement in the behavior-owning repo's specs (or NONE).
3. Appends every newly-passed behavior/evidence pair to its shard's passed-units log (behavior summary + file path + covering requirement) -- one shard log, appended across rounds, never overwritten.
4. Returns its shard's behavior GAP list (behavior contracts with no covering requirement) to the main agent.
The main agent merges the shard GAP lists into one `## Review round {round}` block, APPENDED to coverage.md (never overwrite prior rounds); also append a copy to review.md. Two subagents never touch the same shard log (one subagent per shard), so concurrent appends are safe.

Use the following as your output templates. Follow the structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments.

<template>
## Review round <!-- round -->

Per-repo GAP lists, merged from every shard's subagent. (Passed behavior/evidence pairs live in the per-shard passed-units logs under coverage-passed/, not here -- this block is only the still-uncovered behavior contracts.)

### Repo: <!-- repo path (or "project root") -->

- **Shards reviewed**: <!-- list the shard slugs under this repo that had a subagent this round -->
- **Evidence skipped (already PASS in this shard's log)**: <!-- N, from the passed-units log; M of those re-checked because refine edited their covering spec -->
- **Behavior contracts re-checked this round**: <!-- N -- the prior-round behavior GAPs + refine-edited PASS evidence -->

#### GAP list (uncovered behavior contracts in this repo)

- `<!-- behavior contract summary -->` -> NONE -- <!-- why no covering requirement: missing observable rule / boundary / error semantic / invariant / compatibility surface / ABI / protocol / policy / layout / lifecycle; cite evidence units such as function / trait method / asm routine / linker section / build config / Kconfig / device tree / script / IDL / policy / manifest with file path:line -->
- <!-- "none -- every meaningful behavior contract discovered from this repo's evidence is covered by a requirement" if the gap is empty -->

### Aggregate verdict

- <!-- COVERED (every target repo's behavior gap list is empty) or UNCOVERED (N behavior contracts still lack a covering requirement, across these repos: <list>) -->
</template>

Per-shard passed-units log, at `.openspec-generate-runs/{run}/coverage-passed/<shard-slug>.md` (multi-repo: `<repo-slug>.md`; single-repo: `<module-slug>.md`). Append to it across rounds; never overwrite. Each subagent writes ONLY its own shard's log. The name remains `passed-units` for compatibility, but entries are behavior/evidence pairs.

<template>
# Passed Units -- <!-- shard-slug (repo path or module) -->

Appended across review rounds. A behavior/evidence pair listed here is PASS -- later rounds skip it unless a refine round edited its covering spec (then it is re-checked and, if still covered, re-recorded with the new requirement).

## Round <!-- round -->

For each behavior/evidence pair newly confirmed PASS this round:

- `<!-- behavior contract summary -->` evidenced by `<!-- unit kind / file path:line / symbol or artifact -->` -> `<!-- covering requirement <cap>#<requirement-name> -->`
</template>

<rules>
- LANGUAGE: English output.
- Do NOT fix the gap here -- that is the refine step. This step only senses, records PASS, and reports behavior GAPs.
- Do not report a private helper, source file, build rule, or script as a gap by itself. Report the uncovered behavior contract it reveals. Private/internal parsers and helpers SHOULD produce gaps when they encode standard/protocol bit meanings, field mappings, validation rules, or error semantics that no requirement covers.
</rules>
