---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the processing:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **process** -- fetch, cross-validate, dedup, and log the trail. This is where raw results become a trusted source pool.

Read the `## Round {round} -- raw results` block in `sources.md`. For each result that looks relevant to the scope facets, fetch the page and extract the substance. Then apply three filters and record everything to `log.md` and `sources.md`.

**1. Timeliness.** Is the content's publish/update date acceptable for the question's time frame (see `scope.md` constraints)? A 2018 source on a 2025-state-of-the-art question is likely STALE. Record the date assessment.

**2. Accuracy / authority.** Is the source credible for the claim it makes (official docs, peer-reviewed, established outlet, named expert vs. anonymous blog)? Does the specific claim actually hold up against the source text -- not just the snippet?

**3. Cross-validation.** For key claims (the load-bearing facts the answer will rest on), find at least one OTHER source that confirms or contradicts. Where sources disagree, record the conflict explicitly -- do not silently pick one.

**4. Dedup.** Drop near-duplicates (same content republished, same URL with different tracking params, same paper on multiple mirrors). When two sources cover the same point, keep the more authoritative one and note the duplicate.

After filtering, promote verified entries to a persistent `## Verified sources` section in `sources.md`. Each entry gets a status and a one-line note. Append a `## Round {round} -- processing log` block to `log.md` recording the trail: what was fetched, what verified, what conflicted, what deduped, and which facets the round newly covered.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
## Verified sources

<!-- One entry per source promoted to the trusted pool this round OR updated this round. Each entry carries: title, URL, publish date, the facets it covers, a verification status, and a one-line note. Merge new entries into any existing Verified sources section (do not duplicate sources already listed -- update them instead). -->

### [<!-- title -->](<!-- URL -->)

- **Published**: <!-- date or "date unknown" -->
- **Covers facets**: <!-- which scope facets this source evidences -->
- **Status**: <!-- VERIFIED | PARTIAL | CONFLICT | STALE | UNRELIABLE -->
- **Note**: <!-- one line: what this source establishes, and (if PARTIAL/CONFLICT/STALE/UNRELIABLE) why it is not fully VERIFIED -->
- **Cross-checked against**: <!-- the other source(s) that confirm or contradict this one, or "standalone -- no corroboration found" -->

<!-- ...one entry per promoted source... -->

## Round <!-- N --> — processing log

<!-- A compact trail of this round's processing, so the run is auditable. -->

- **Fetched**: <!-- count + notable pages fetched -->
- **Verified**: <!-- which claims/sources reached VERIFIED this round -->
- **Conflicts**: <!-- claim-level disagreements found between sources, or "none" -->
- **Deduped**: <!-- near-duplicates dropped (title/URL of the kept vs dropped), or "none" -->
- **Facets newly covered**: <!-- which scope facets this round added verified backing for, or "none -- reinforced existing coverage" -->
- **Skipped**: <!-- results fetched but discarded as irrelevant/low-quality, with why -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Fetch the page before judging it. A snippet alone is not enough to assess accuracy or extract substance.
- The five statuses are exhaustive and mutually exclusive per entry:
  - VERIFIED: credible source, claim holds, timeliness OK, corroborated where required.
  - PARTIAL: useful but incomplete (e.g. covers one sub-claim, or corroboration partial).
  - CONFLICT: the claim contradicts at least one other source; record both sides.
  - STALE: content is too old for the question's time frame, even if otherwise credible.
  - UNRELIABLE: source lacks credibility for the claim (anonymous, SEO content, no provenance).
- Cross-validation is for KEY claims -- the load-bearing facts, not every incidental detail. Use judgment; do not block on finding a second source for trivial facts.
- When sources conflict, never silently pick one. Record the conflict in the entry's note and the processing log; the report will flag it.
- Dedup keeps the more authoritative instance. "More authoritative" beats "first found."
- Append the Verified sources entries and the processing log to their respective files -- do not overwrite prior rounds. If a source was already verified in an earlier round, update its entry rather than re-adding it.
- Execute only this instruction.
</rules>
