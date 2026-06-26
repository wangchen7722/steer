---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the synthesized report:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **synthesize** -- write the final cited report from the verified source pool.

Read `scope.md` (the facets a thorough answer must cover and the success criterion), `sources.md` (the verified source pool -- each entry has a status and a note), and `log.md` (the round-by-round trail of what was covered, conflicted, and left open). Synthesize `report.md`: a structured answer to the question, organized by the scope facets, drawing only on the verified sources.

Citation discipline:
- Cite inline at the point of each claim (author/title/URL), preferring VERIFIED sources.
- Where the backing is PARTIAL, CONFLICT, STALE, or UNRELIABLE, say so at the claim rather than presenting it as settled fact.
- Where sources conflicted, present the conflict and the evidence on each side; do not paper it over.
- State the limits of what the research could establish -- the question's edges the evidence did not reach.
- List the verified sources used as a references section.

The report's honesty about coverage is mandatory. `covered` tells you which shape to take: when covered=true, all important facets had adequate verified backing. When covered=false, the loop hit the round cap with residual gaps -- include an explicit "Unresolved / under-covered" section listing the facets still lacking evidence, so the reader knows exactly where the answer is thin.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
# Research Report — <!-- run name -->

## Question

<!-- Restate the research question. -->

## Summary

<!-- 3-5 lines: the direct answer to the question, with the most important findings up front. This is what a reader who only reads the top takes away. -->

## Findings by facet

<!-- One section per scope facet. Under each, present what the verified sources establish, citing inline. Use the facet name from scope.md as the heading so coverage is auditable against the scope. -->

### <!-- facet name -->

<!-- Findings with inline citations. Flag PARTIAL/CONFLICT/STALE evidence where it applies. -->

### ...

## Conflicts and caveats

<!-- Where sources disagreed, or where evidence is thin/contested, lay it out here. If there are none, state "No material conflicts among verified sources." Do not manufacture consensus the sources did not provide. -->

## Limits of this research

<!-- What the research could NOT establish -- facets where evidence was insufficient, time frames not covered, perspectives missing. Honest about the edges. -->

## Unresolved / under-covered

<!-- ONLY when covered=false. List the facets still lacking verified evidence, with what is missing and what a further round would target. Omit this entire section when covered=true. -->

## References

<!-- The verified sources used in this report. One entry per source: title, author (if known), URL, publish date, status. List only sources actually cited above. -->

- <!-- title --> — <!-- author or "author unknown" --> — <!-- URL --> — <!-- date --> — <!-- status -->
- ...
</template>

<rules>
- LANGUAGE: Write all output in English.
- Cite inline at the point of each claim. Prefer VERIFIED sources; if a claim rests only on PARTIAL/CONFLICT/STALE/UNRELIABLE evidence, say so at that claim.
- Never present contested evidence as settled. Where sources conflict, present both sides with their citations.
- Organize findings by the scope facets (use scope.md's facet names as headings) so coverage is auditable against the scope.
- The "Unresolved / under-covered" section appears ONLY when covered=false. When covered=true, omit it entirely.
- The references section lists only sources actually cited in the report -- not every source harvested.
- Be honest about limits. A report that overstates what the evidence supports is worse than one that names its gaps.
- Execute only this instruction.
</rules>
