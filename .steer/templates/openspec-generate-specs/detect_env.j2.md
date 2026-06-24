---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the environment detection:
  <check>{{ check }}</check>
---
**detect-env**: detect the project environment and build the target-repo inventory ONCE, so every later step branches on facts rather than re-detecting. This step does NOT gather context or write specs.

**Multi-repo rule (safety-critical):** specs go into the `openspec/specs/` of a real git repo (a *target repo*). In a `repo`-manifest checkout the project root is NOT a git repo — it MUST NOT receive an `openspec/` directory (untracked, wiped by the next `repo sync`); the target repos are the sub-repos. In a single-repo project the project root is the one target repo.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Fill in the output template below, replacing each `<!-- ... -->` with real content and removing the placeholder.

<template>
# Spec Generation Environment: <!-- run name -->

## Repo topology

- **Type**: <!-- SINGLE-REPO or MULTI-REPO -->
- **Evidence**: <!-- how you determined the type -->

## Target-repo inventory

For EACH target repo (every sub-repo in a multi-repo checkout; the project root in a single repo):

### Target repo: <!-- path -->

- **Path**: <!-- repo path -->
- **Git remote**: <!-- origin URL -->
- **Git host**: <!-- e.g. gitblueos.vivo.xyz or github.com -->
- **Project path**: <!-- group/project, e.g. BlueOS/Kernel/BlueKernel/kernel -->
- **Host CLI**: <!-- glab or gh, installed + authed? -->
- **CodeGraph**: <!-- PRIMARY or NOT-AVAILABLE. PRIMARY when the CLI is installed AND an index covers this repo: at the manifest root for MULTI-REPO (keeps cross-repo relationships visible), at the project root for SINGLE-REPO. Record where the index lives. NOT-AVAILABLE (CLI missing or no covering index) -> fall back to grep/find/Read for this repo. -->
- **openspec/specs/ exists?**: <!-- YES / NO -->
- **Existing capability names (coverage floor)**: <!-- list every directory under this repo's openspec/specs/ verbatim; these names MUST be reproduced in this repo's regenerated output. "none (bootstrap)" if absent -->
- **changes/archive/ exists?**: <!-- YES / NO -->
- **Archived change deltas**: <!-- entries under openspec/changes/archive/; "none" if absent -->

## Run type

- <!-- BOOTSTRAP (no target repo had prior openspec/specs/) or REFRESH (at least one did) -->
</template>

<rules>
- LANGUAGE: write all output in English.
- A target repo's existing `openspec/specs/` capability names are its coverage floor — record them verbatim; a refresh must reproduce every one under the same name, in that repo.
- This step only detects and records the environment + inventory. Do not gather context or write specs.
</rules>
