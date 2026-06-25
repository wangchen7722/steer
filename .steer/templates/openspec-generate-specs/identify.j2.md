---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the finalized capability list:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **identify** -- finalize the capability list AND assign each a primary-owner repo plus its participating repos. Read EVERY shard under capabilities/ (the gathered context, one per repo/module); merge them into one global view and write the FINAL list to final-list.md (this step does not overwrite the shards — it reads them and produces a new consolidated file). Write incrementally, not all at once: a large repo can yield hundreds or thousands of capabilities, and one giant write is error-prone and loses everything if it fails mid-way. Write the file header (the `## Decomposition rules applied` block) first, then append capabilities in batches of ~20, flushing each batch to disk before starting the next; finalize any summary/totals at the end.

The decomposition rule is the heart of spec quality. Split by SINGLE MECHANISM / single responsibility, NOT by source directory or module domain:

- WRONG (directory-domain): "sim-management" bundling ICCID parsing, PIN lock, state machine, and file IO into one spec.
- RIGHT (mechanism): `sim-state-machine` (the status transition logic), `sim-pin-lock` (the PIN/PUK sync bridge), `sim-file-io` (the EF read/write engine), `sim-core-files` (the specific file decode rules).
- When one source directory implements several distinct mechanisms, that is SEVERAL capabilities. When one mechanism spans several directories or repos, that is ONE capability.
- Archived change deltas are a granularity SIGNAL, not a split key: a delta's name and scope suggest how big a mechanism is and what to call it.
- A capability name should describe a stable behavior contract, not a helper cluster or implementation shape. Good OS/Android boundaries include driver ABI, service lifecycle, protocol state machine, media pipeline stage, framework API surface, permission/policy boundary, boot/power lifecycle, build-selected compatibility behavior, device-tree binding, or linker/assembly ABI.

SPECS FOLLOW CODE AS EVIDENCE. For each capability, ASSIGN A PRIMARY-OWNER REPO (the home of the end-to-end/orchestration spec -- the repo with the majority of the code or the feature's natural home) AND LIST ITS PARTICIPATING REPOS (every repo whose code or contract-bearing artifacts implement part of this capability, including the owner). Each participating repo gets its OWN `<cap>/spec.md` under the SAME capability name -- capability names are per-repo-local, so the same name in two repos is two independent specs, NOT a conflict. The owner's spec carries the end-to-end requirements; each participant's spec describes that repo's own side. When a capability's code lives entirely in one repo, the participating-repos list is just [that repo] and it gets a single spec. Related specs (a cross-repo participant of the SAME capability, or a DIFFERENT capability in any repo) are referenced INLINE in requirement descriptions as context; the exact link syntax is pinned in the spec-writing task, not here. A purely-supporting repo whose code or configuration participates in a cross-repo capability gets its own local-side spec in that repo -- it is NEVER left with zero specs.

Do NOT finalize capabilities by copying source directory names or private function names. The final list should preserve stable, testable behavior contracts: observable behavior, boundaries, error semantics, invariants, compatibility surfaces, ABI/protocol/policy/layout/lifecycle rules, and the implementation evidence that supports them.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
# Final Capability List: <!-- run name -->

## Decomposition rules applied

- Split by single mechanism/responsibility, not by source directory.
- Reused every existing capability name verbatim, per its repo (none renamed/merged/dropped).
- Each capability assigned a primary-owner repo + a participating-repos list; specs follow code (each participating repo gets its own <cap>/spec.md under the same name).
- Every name is kebab-case (lowercase letters, digits, hyphens only).

## Capabilities

For each FINAL capability:

### Capability: <!-- kebab-case name -->

- **Primary-owner repo**: <!-- the repo whose spec carries the end-to-end/orchestration requirements. Single-repo: "project root". Multi-repo: the participating-repo path that owns the end-to-end story -->
- **Participating repos**: <!-- every repo whose code implements part of this capability, INCLUDING the owner. Single-repo or code-in-one-repo: just "[<that repo>]". Multi-repo: list each participating repo + the cross-repo signal (Depends-On MR, close merge timestamps, code/doc cross-references) -->
- **Mechanism / behavior contract**: <!-- one-line: the single mechanism and stable behavior contract this captures -->
- **Per-repo REUSE-vs-NEW**: <!-- for EACH participating repo: REUSE-existing (openspec/specs/<name>/ already existed in THAT repo per env.md) or NEW. A capability may be REUSE in one repo and NEW in another -->
- **Per-repo behavior contracts to preserve**: <!-- for EACH participating repo, the observable behavior, boundary conditions, error semantics, invariants, state transitions, compatibility surfaces, ABI/protocol/policy/layout/lifecycle rules that THIS repo implements under this capability. The owner repo additionally carries the end-to-end/orchestration behavior -->
- **Per-repo implementation evidence**: <!-- for EACH participating repo, the source/config/build/script/linker/assembly/test/history evidence supporting the behavior. Keep evidence separate from requirements unless the exact identifier/value/path/signature is itself a compatibility contract -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- EVERY capability MUST have a primary-owner repo AND a participating-repos list recorded — one <cap>/spec.md will be written per participating repo later.
- Specs follow code as evidence: each participating repo gets its OWN `<cap>/spec.md` under the SAME capability name. The owner's spec carries end-to-end requirements; each participant's spec carries that repo's local-side requirements. Related specs are linked INLINE in requirement descriptions as context (cross-repo participant or a different capability, same/cross-repo).
- Capability names and boundaries must be behavior-contract oriented, not source-directory, helper-function, or private implementation oriented.
- When prior openspec/specs/ existed in a repo (per env.md), every existing capability name in THAT repo MUST appear here under the SAME name marked REUSE-existing in that repo. Renaming, merging, or dropping an existing capability is FORBIDDEN -- add NEW capabilities only for genuinely unspecced behavior.
- A capability's REUSE-vs-NEW status is PER REPO, not per capability -- it may be REUSE in one repo and NEW in another.
- Reject any name that is not kebab-case. The name becomes the spec directory and the `openspec validate <name> --type spec` target, used identically in every participating repo.
- This is the FINAL list — spec writing will iterate exactly these capabilities + participating repos, so be complete.
</rules>
