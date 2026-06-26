---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the planned queries:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **plan_queries** -- decide this round's search suggestions before fan-out.

Read `scope.md` for the facets a thorough answer must cover. On round 2+, also read `log.md` to see which facets prior rounds already covered with verified sources and the expansion seeds the previous round left. Then generate the search suggestions for THIS round.

The goal is a set of **different but related** web-search keywords/phrases. Balance two forces:
- **Breadth**: target facets that are still uncovered or under-covered.
- **Depth**: go deeper on the highest-priority facet (more specific, more technical, more authoritative phrasings).

Each suggestion must be a concrete web-search string an agent can paste into a search engine -- not a topic description. Vary phrasing across suggestions so parallel searches surface different sources (synonyms, specific entities, technical terms, "vs"/comparison forms, site-targeted forms like `site:arxiv.org`).

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
## Round <!-- N --> — planned queries

<!-- One numbered query per line. Each is a concrete web-search string. After each, a one-line note on which facet it targets and whether it is for BREADTH or DEPTH. -->

1. `<!-- concrete search string -->` — <!-- facet targeted; BREADTH or DEPTH -->
2. `<!-- concrete search string -->` — <!-- facet targeted; BREADTH or DEPTH -->
...

### Rationale

<!-- 2-3 lines: which facets this round prioritizes and why (referencing what prior rounds covered, on round 2+), and how the set balances breadth vs depth. -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Each suggestion is a concrete, paste-into-a-search-engine string -- never a vague topic. "rust async runtime" is fine; "something about async" is not.
- Vary phrasing across the set so parallel searches diverge rather than all returning the same top hit.
- Prioritize still-uncovered facets first, then deeper cuts on the priority facet. Do not re-query facets already well-covered with VERIFIED sources.
- Do NOT search the web here. This step only writes the plan; the next step fans out the searches.
- Append this block to log.md -- do not overwrite prior rounds' blocks.
- Execute only this instruction.
</rules>
