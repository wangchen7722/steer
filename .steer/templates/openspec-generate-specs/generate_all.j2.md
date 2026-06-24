---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the generated specs:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **generate-all** -- write or merge EVERY capability's main spec, ONE `<cap>/spec.md` PER participating repo, and validate each in place. Read final-list.md for the FINAL list -- each capability has a primary-owner repo, a participating-repos list, per-repo REUSE-vs-NEW, and per-repo contract facts.

SPECS FOLLOW CODE. For EACH capability, iterate its participating repos and write one spec file per repo. FAN OUT, but NOT one subagent per (capability, repo) — a large repo yields hundreds or thousands of capabilities, and that many subagents is the wrong shape. Fan out by SHARD instead: one subagent per module shard (multi-repo: per repo; single-repo: per top-level module), and each subagent writes EVERY spec for the capabilities whose code lives in its shard, serially within the shard. The shard boundaries already exist — they are the capabilities/ shards from the gather step, and the per-repo grouping from env.md — so reuse them; do not re-invent a partition here. A shard's subagent writes each `<R>/openspec/specs/<cap>/spec.md` under its repo(s) and validates it in place; the one serialization rule is that two subagents must not edit the SAME spec file at once (only possible when both are MERGEs into one existing spec that straddles shards) — route such a file to one subagent. For EACH (capability, repo R):

**A. Write/merge the spec** at `<R>/openspec/specs/<cap>/spec.md` (in a single-repo project `<R>` is the project root; in a multi-repo checkout it is the participating sub-repo. NEVER write to the manifest root):
- If R is the **primary-owner**: the spec carries the end-to-end/orchestration requirements (the cross-repo story, state machine, contract) PLUS R's own local-side requirements.
- If R is a **non-owner participant**: the spec carries R's own local-side requirements only -- what R's code does under this capability.
- **Inline links to related specs (context only):** reference related specs INLINE in the requirement description where the relationship matters. Cross-repo link (path relative to the manifest root, anchored on the sub-repo dir name): `[<cap> in <repo-name>](<repo-name>/openspec/specs/<cap>/spec.md)`. Same-repo link (path relative to THIS repo's root, no repo prefix): `[<cap>](openspec/specs/<cap>/spec.md)`. Both cross-repo participants of the SAME capability and DIFFERENT capabilities (in the same repo or another repo) use this same inline form -- links live inside the requirement descriptions they support. BIDIRECTIONAL for cross-repo participants of the SAME capability: when a capability spans repos, every participating repo's spec links to its cross-repo partners and vice versa -- at minimum the owner spec links to each participant spec and each participant spec links back to the owner spec, so the relationship is discoverable from either side. Other related-spec links (a DIFFERENT capability) stay one-directional and on-demand.
- If the spec already exists (REUSE-existing in R): MERGE -- preserve every existing requirement/scenario/contract fact VERBATIM and ADD what is missing. Never drop or rename existing content. Only add.
- If it does not exist (bootstrap or NEW in R): create it fresh.

**B. Validate in place** right after writing: `cd <R> && openspec validate <cap> --type spec`. Fix and re-run until exit 0 with no ERROR lines. Common fixes: `Requirement must contain SHALL or MUST keyword` -> use SHALL/MUST (remove should/may); `Requirement must have at least one scenario` -> add a `#### Scenario:` block with exactly four hashtags; `Purpose section is too brief` -> expand the `## Purpose` text.

Single-repo capability (participating == [project root]): one spec file, no cross-repo links -- identical to the plain single-repo path.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
The OpenSpec main-spec file you write for EACH (capability, repo) follows this structure. Follow it exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments. On a MERGE, keep every existing `### Requirement:` and its scenarios in place and add new ones below; do not rewrite the existing ones. Every repo's spec must stand on its own -- real SHALL/MUST requirements describing THAT repo's observable behavior.

<template>
# <!-- Capability Name (Title Case) -->

## Purpose

<!-- What this capability does and why it exists, scoped to THIS repo's role under the capability. At least 50 characters. Grounded in actual code behavior, not aspiration. -->

## Requirements

### Requirement: <!-- Requirement Name -->

<!-- The system SHALL/MUST <behavior>. ONE concern per requirement. This requirement describes the behavior of one or more FUNCTIONAL UNITS in THIS repo (a function of any visibility, trait method, assembly routine, or linker-script SECTIONS block / symbol -- not only public/exported units). Pin THIS repo's concrete contract facts here or in the scenarios: identifiers + numeric values, timeouts, enum values + their integers, return/error codes, exact signatures, trait boundaries, state transitions; for linker/asm units, section names + addresses + alignment + symbol names + vector offsets. Implemented in <source path relative to THIS repo's root>. One requirement may cover several functional units that share this behavior -- do not create one requirement per unit. Where a related spec clarifies this requirement, reference it INLINE here as context: a cross-repo participant `[<cap> in <repo>](<repo>/openspec/specs/<cap>/spec.md)` or a same-repo capability `[<cap>](openspec/specs/<cap>/spec.md)`. -->

#### Scenario: <Scenario Name>
- **WHEN** <action or condition>
- **THEN** <expected outcome>
- **AND** <additional outcome if needed>

#### Scenario: <Another Scenario>
- **WHEN** <action or condition>
- **THEN** <expected outcome>

<!-- For the PRIMARY-OWNER repo only (and only when this capability has more than one participating repo): additional end-to-end/orchestration requirements here, describing how the capability spans repos. Each end-to-end requirement must ALSO contain SHALL/MUST + a scenario, and MAY reference a cross-repo participant inline in its description. -->
</template>

Record EVERY (capability, repo) to generation-log.md using this structure (one entry per pair). The log can hold hundreds or thousands of entries, so write it incrementally — do not hold the whole log in memory and dump it once at the end. Write the `## Generation log:` header first, then append entries as pairs finish. Coordinate with the fan-out: each shard's subagent RETURNS its shard's log entries (the filled structure below, one per pair it wrote) to the main agent, and the main agent appends the returned entries (one shard's batch, or ~20 at a time) to generation-log.md — do NOT let subagents append to the shared log directly, since concurrent appends to one file lose entries.

<template>
## Generation log: <!-- run name -->

### Capability: <!-- kebab-case name --> / Repo: <!-- repo path; "project root" for single-repo; NEVER the manifest root -->

- **Role**: <!-- primary-owner or participant -->
- **Spec path written**: <!-- <repo>/openspec/specs/<cap>/spec.md -->
- **Related specs linked inline?**: <!-- list each inline link emitted in this spec (cross-repo `<cap> in <repo>` or same-repo `<cap>`), or "none" -->
- **Requirements**: <!-- N -->
- **Scenarios**: <!-- M -->
- **Status**: <!-- New / Merged -->
- **Per-(cap,repo) validate**: <!-- `cd <repo> && openspec validate <cap> --type spec` -> exit 0 / fixed ERROR then exit 0 -->
</template>

<rules>
- LANGUAGE: Write all output in English. Code/identifiers follow existing project conventions.
- `openspec validate` does NOT follow links -- links are human-readable context only; link rot never breaks validation and must never gate a boolean.
- Trivial no-transform units (pure getter/setter passthrough, single constant assignment, pure formatting wrapper) need no dedicated requirement -- their behavior is that of the requirement they participate in.
- Execute only this instruction. Validate each spec SINGLY in place (`openspec validate <cap> --type spec`); the full whole-repo `openspec validate --specs` pass runs once, later, after the review<->refine loop -- not here.
</rules>
