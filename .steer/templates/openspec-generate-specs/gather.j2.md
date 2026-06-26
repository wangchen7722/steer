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
This is the openspec-generate-specs workflow. Current position: **gather** -- deep-mine the capabilities. The scan plan already grouped behavior-bearing files/artifacts into deep-scan units by code semantics, so fan out one deep-mine subagent per scan unit -- do NOT re-split units by file count; each subagent mines its unit itself and returns. env.md holds the per-repo branching (CodeGraph vs grep, host CLI, coverage floor).

Each deep-mine subagent works its unit across all sources below, returns candidate capabilities each with participating-repos + per-repo behavior contracts and implementation evidence partitioned BY repo, then the main agent merges all units' results, dedupes cross-unit/cross-repo capabilities, and writes the results SHARDED — never one giant file (a large repo's context would overflow a single file). Shard by repo for a multi-repo checkout and by top-level module for a single repo; the per-shard path and structure are in the output template below. A cross-repo unit (noted in the plan, with its seam) is mined by ONE subagent that reads both sides together — but its output still splits BY repo: each repo's behavior contracts and evidence go into that repo's shard, and the cross-repo relationship is recorded as a bidirectional inline link: A's shard links to B's spec (A calls B) and B's shard links back to A's spec (B is called by A), never as one merged spanning spec.

Mine these sources PER target repo. The running code is FIRST — it is the ground truth for what the system actually does, and specs are generated FROM it, so when code and a prior spec/delta conflict, the code wins by default and the spec is flagged stale. Only treat the spec/delta as overriding the code when there is strong evidence the code is a bug (e.g. the spec/delta + tests + commit history all say one thing and the code alone deviates). Flag every divergence either way. Sources, in priority: (1) actual behavior via CodeGraph (explore/node/callers) or grep/find/Read -- code is implementation evidence, not prose to paraphrase; (2) OS/Android contract-bearing files such as Kconfig, defconfig, device tree, boot parameters, `BUILD.gn`, `Android.bp`, `Android.mk`, GN args, product/board configuration, generated config headers, init rc, system properties, SELinux policy, permission declarations, AIDL/HIDL/Binder interfaces, service registration, HAL manifests, compatibility matrices, linker scripts (`link.x`, `.ld`, `.lds`), assembly, and Python/shell/generator scripts when they define APIs, protocol tables, packaging, or compatibility checks; (3) existing OpenSpec artifacts in that repo (its own openspec/specs/ + changes/archive/ deltas -- the reviewed record of what was intended, useful for rationale and behavior contracts the code buries); (4) test assertions; (5) BDD .feature files / design docs; (6) MRs via glab/gh; (7) full git commit history (the ENTIRE log -- a behavior's rationale may live only in its original commit, but current behavior is set by the latest commit, so read both ends); (8) README/AGENTS.md; (9) referenced industry standards, protocol definitions, Android CDD/VTS/CTS rules, and vendor interface documents when they are cited by code, tests, generated artifacts, or docs.

SPECS FOLLOW CODE AS EVIDENCE: for each candidate capability, record every repo whose code or contract-bearing artifacts implement part of it (the participating repos), and PARTITION the mined behavior contracts and implementation evidence BY the repo whose side they describe — each repo's side must land in its own shard, so later per-repo spec writing can pick up exactly that repo's behavior. A repo that contributes code or contract-bearing configuration to a capability, even only as a supporting participant, must surface its local-side behavior here.

SPEC QUALITY: extract stable, verifiable behavior contracts. Do NOT merely translate source code into natural-language implementation summaries. A private function, helper, parser, local variable, or call sequence is evidence only when it is just implementation shape; it becomes spec-worthy when it defines or enforces an observable rule, industry-standard parsing semantic, bit/field meaning, ABI, protocol, persisted format, policy, layout, lifecycle, compatibility behavior, state transition, or externally observable error semantic.

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
- **Code and contract artifacts**: <!-- CodeGraph explore/node/callers when env.md marks this repo PRIMARY, else grep/find/Read; in multi-repo query the manifest-root index so cross-repo relationships are visible. Include source, linker scripts (`link.x`, .ld/.lds), assembly (.S/.s/.asm), Kconfig/defconfig/device tree, BUILD.gn/Android.bp/Android.mk/GN args/product/board config/generated headers, init rc/system properties, SELinux/permissions, AIDL/HIDL/Binder interfaces, HAL manifests/compatibility matrices, and scripts/generators when they define behavior. -->
- **MRs**: <!-- MRs read for this repo via the host CLI; note if skipped (CLI missing/unauthed) -->
- **Existing specs**: <!-- every openspec/specs/<cap>/spec.md read in full for this repo -->
- **Git history**: <!-- full `git -C <repo> log --all` for this repo, bodies included -->
- **Archived deltas**: <!-- each openspec/changes/archive/<change>/ proposal.md + design.md + specs/<cap>/spec.md read for this repo -->

## Candidate capabilities (this repo/module's side)

For each candidate capability whose code or contract-bearing artifacts live partly or wholly in this repo/module (one mechanism / one responsibility; archived deltas are a granularity SIGNAL only -- the mapping is NOT 1:1, several deltas may evolve the SAME mechanism, so do NOT split one capability per delta):

### Candidate: <!-- kebab-case name -->

- **Mechanism**: <!-- one-line: the single mechanism/state-machine/contract/lifecycle this captures -->
- **Participating repos**: <!-- every repo whose code or contract-bearing artifacts implement part of this capability. Single-repo: "[project root]". Multi-repo: list each repo that contributes code/config/artifacts; if it spans repos, note the cross-repo signal (Depends-On MR, close merge timestamps, code/doc cross-references). A repo contributing only supporting code/config/artifacts is still a participant -->
- **REUSE-existing or NEW here**: <!-- REUSE-existing if openspec/specs/<name>/ already exists in THIS repo (per env.md), else NEW -->
- **Behavior contracts to preserve (THIS repo's side only)**: <!-- the stable behavior THIS repo implements under this capability. Do NOT include another repo's side behavior here — it belongs in that repo's shard. Separate:
  - Observable behavior: inputs, outputs, state changes, externally visible effects
  - Boundary conditions: invalid inputs, limits, defaults, feature gates, fallback paths
  - Error semantics: return codes, errno/status mappings, exceptions, retry/cancel/timeout behavior
  - Invariants/state machines: states, transitions, ordering, lifetime, ownership, concurrency, power/boot/service lifecycle
  - Compatibility surfaces: public API, kernel/userspace ABI, Binder/AIDL/HIDL/HAL contract, ioctl/sysfs/procfs/debugfs/netlink ABI, protocol/wire format, persisted format, media/Bluetooth/Wi-Fi/telephony/USB/PCIe/UFS/eMMC/NVMe obligations, Android CDD/VTS/CTS behavior, SELinux/init/property/manifest policy, device tree binding, Kconfig/BUILD.gn/Blueprint/Make/product-selected behavior, linker/assembly layout or entry ABI
  If this repo holds the end-to-end orchestration, also record the end-to-end/orchestration behavior here -->
- **Implementation evidence (THIS repo's side only)**: <!-- source paths, functions/methods, constants, enum values, signatures, config files, scripts, linker/assembly sections, tests, commits, MRs, and docs that evidence the behavior above. These are evidence, not requirements, unless changing them would break an external contract or compatibility promise. -->
- **Cross-repo relationships (if any)**: <!-- for a cross-repo capability: the bidirectional inline link — THIS repo's shard links to the other repo's spec (direction: calls / is-called-by), and the other repo's shard links back (reverse direction). Also note the seam this repo participates in (which symbols call/depend on or are called by the other repo). Omit for single-repo capabilities -->
- **Evidence sources**: <!-- which sources evidenced each behavior contract and evidence item above -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Per target repo: mine the ENTIRE git history of that repo, not a recent window. The original commit that added a behavior often states its rationale/parameters/constants more explicitly than the code does now; the latest commit sets the current behavior. Read both ends.
- When the code and a prior spec/delta disagree, the code wins by default (specs follow code) — flag the spec as stale, unless the spec/delta + tests + history together show the code is a bug. Never silently pick one.
- Capture behavior contracts as explicit items, not prose. "SHALL handle appropriately" is a defect -- if a fact is unknown after searching, state the observable behavior precisely and flag the gap.
- Keep implementation evidence separate from requirements. Identifiers, constants, signatures, paths, build rules, scripts, and helper functions are pinned as SHALL/MUST only when they are part of an external contract, compatibility surface, ABI, protocol, policy, persisted format, layout, lifecycle, or externally observable error semantic.
- For each candidate capability, record the participating repos AND partition behavior contracts and implementation evidence BY repo, each side into its own shard. A supporting repo must surface its local-side behavior in its own shard, not be folded silently into another repo's.
- Do NOT finalize the capability list or assign owners here — this step gathers and synthesizes context per candidate; finalizing and ownership are a later, separate task.
</rules>
