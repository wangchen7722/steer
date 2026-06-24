---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the gathered context:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **gather** -- deep-mine the capabilities. The scan plan already grouped source files into deep-scan units by code semantics, so fan out one deep-mine subagent per scan unit -- do NOT re-split units by file count; each subagent mines its unit itself and returns. env.md holds the per-repo branching (CodeGraph vs grep, host CLI, coverage floor).

Each deep-mine subagent works its unit across all six sources (below), returns candidate capabilities each with participating-repos + per-repo contract facts partitioned BY repo, then the main agent merges all units' results, dedupes cross-unit/cross-repo capabilities, and writes the results SHARDED — never one giant file (a large repo's context would overflow a single file). Shard by repo for a multi-repo checkout and by top-level module for a single repo; the per-shard path and structure are in the output template below. A cross-repo unit (noted in the plan, with its seam) is mined by ONE subagent that reads both sides together — but its output still splits BY repo: each repo's contract facts go into that repo's shard, and the cross-repo relationship (A calls B) is recorded as an inline link from the calling repo's shard toward the other repo's spec, never as one merged spanning spec.

Mine these sources PER target repo. The running code is FIRST — it is the ground truth for what the system actually does, and specs are generated FROM it, so when code and a prior spec/delta conflict, the code wins by default and the spec is flagged stale. Only treat the spec/delta as overriding the code when there is strong evidence the code is a bug (e.g. the spec/delta + tests + commit history all say one thing and the code alone deviates). Flag every divergence either way. Sources, in priority: (1) actual code behavior via CodeGraph (explore/node/callers) -- ground truth; (2) existing OpenSpec artifacts in that repo (its own openspec/specs/ + changes/archive/ deltas -- the reviewed record of what was intended, useful for rationale and contract facts the code buries); (3) test assertions; (4) BDD .feature files / design docs; (5) MRs via glab/gh; (6) full git commit history (the ENTIRE log -- a behavior's rationale may live only in its original commit, but current behavior is set by the latest commit, so read both ends); (7) README/AGENTS.md.

SPECS FOLLOW CODE: for each candidate capability, record every repo whose code implements part of it (the participating repos), and PARTITION the mined contract facts BY the repo whose code they describe — each repo's side must land in its own shard, so later per-repo spec writing can pick up exactly that repo's facts. A repo that contributes code to a capability, even only as a supporting participant, must surface its local-side facts here.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
One file per shard, at `capabilities/<repo-slug>/<module-slug>.md` (multi-repo) or `capabilities/<project-root-slug>/<module-slug>.md` (single-repo). Each shard covers ONE repo (multi-repo) or ONE top-level module (single-repo) and holds only the candidate capabilities whose code lives in that repo/module's side. Use this structure for every shard:

# Gathered Context — <!-- repo-slug / module-slug -->

## Sources consulted for this shard

- **Docs**: <!-- AGENTS.md/CLAUDE.md/README.md, docs/, book/, .feature files read for this repo/module -->
- **Code**: <!-- CodeGraph explore/node/callers when env.md marks this repo PRIMARY, else grep/find/Read; in multi-repo query the manifest-root index so cross-repo relationships are visible. "Code" includes linker scripts (.ld/.lds) and assembly (.S/.s/.asm) — list them here too -->
- **MRs**: <!-- MRs read for this repo via the host CLI; note if skipped (CLI missing/unauthed) -->
- **Existing specs**: <!-- every openspec/specs/<cap>/spec.md read in full for this repo -->
- **Git history**: <!-- full `git -C <repo> log --all` for this repo, bodies included -->
- **Archived deltas**: <!-- each openspec/changes/archive/<change>/ proposal.md + design.md + specs/<cap>/spec.md read for this repo -->

## Candidate capabilities (this repo/module's side)

For each candidate capability whose code lives partly or wholly in this repo/module (one mechanism / one responsibility; archived deltas are a granularity SIGNAL only -- the mapping is NOT 1:1, several deltas may evolve the SAME mechanism, so do NOT split one capability per delta):

### Candidate: <!-- kebab-case name -->

- **Mechanism**: <!-- one-line: the single mechanism/state-machine/contract/lifecycle this captures -->
- **Participating repos**: <!-- every repo whose code implements part of this capability. Single-repo: "[project root]". Multi-repo: list each repo that contributes code; if it spans repos, note the cross-repo signal (Depends-On MR, close merge timestamps, code/doc cross-references). A repo contributing only supporting code is still a participant -->
- **REUSE-existing or NEW here**: <!-- REUSE-existing if openspec/specs/<name>/ already exists in THIS repo (per env.md), else NEW -->
- **Contract facts to pin (THIS repo's side only)**: <!-- the facts THIS repo's code implements under this capability, mined verbatim from deltas + code + tests. "Code" includes linker scripts and assembly — mine them too, not just .rs/.c/.cpp. Do NOT include another repo's side facts here — they belong in that repo's shard. Cover:
  - Identifiers + numeric values: message/command IDs, file IDs/addresses, ports
  - Numeric thresholds/timeouts
  - Enum/constant values: variant name AND integer value
  - Return/error codes + mappings
  - Exact signatures + parameter order
  - Abstraction/trait boundaries: trait + implementors + injection point
  - State machines: states + the transitions actually enforced
  - Linker-script facts: SECTIONS layout, PROVIDE/symbol definitions, addresses/regions (from .ld/.lds)
  - Assembly facts: entry/vector-table layout, routine entry points, calling convention (from .S/.s/.asm)
  If this repo holds the end-to-end orchestration, also record the end-to-end/orchestration facts here -->
- **Cross-repo relationships (if any)**: <!-- for a cross-repo capability: the inline link from THIS repo's side to the other repo's spec, and the seam this repo participates in (which symbols call/depend on the other repo). Omit for single-repo capabilities -->
- **Evidence sources**: <!-- which sources evidenced each fact above -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Per target repo: mine the ENTIRE git history of that repo, not a recent window. The original commit that added a behavior often states its rationale/parameters/constants more explicitly than the code does now; the latest commit sets the current behavior. Read both ends.
- When the code and a prior spec/delta disagree, the code wins by default (specs follow code) — flag the spec as stale, unless the spec/delta + tests + history together show the code is a bug. Never silently pick one.
- Capture contract facts as explicit items, not prose. "SHALL handle appropriately" is a defect -- if a fact is unknown after searching, state the observable behavior precisely and flag the gap.
- For each candidate capability, record the participating repos AND partition the contract facts BY repo, each side into its own shard. A supporting repo must surface its local-side facts in its own shard, not be folded silently into another repo's.
- Do NOT finalize the capability list or assign owners here — this step gathers and synthesizes context per candidate; finalizing and ownership are a later, separate task.
</rules>
