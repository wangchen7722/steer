---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the expansion seeds:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **expand** -- derive the next round's search direction from the most relevant results found so far. This runs only when the round was NOT covered, to push depth and close breadth gaps.

Read `log.md` (the covered facets and this round's processing log) and `sources.md` (the verified sources and the most relevant raw results). From the substance already surfaced, derive follow-up search suggestions that the next round's `plan_queries` will consume.

Two kinds of seeds, mixed:
- **Depth seeds**: go deeper on promising threads -- specific entity names, named techniques/algorithms, author or institution names, a benchmark or dataset mentioned, a sub-mechanism worth unfolding. These follow a thread to its source.
- **Breadth seeds**: open still-uncovered facets or adjacent/competing viewpoints surfaced by the research so far -- an alternative approach, a counter-argument, a related subtopic the results hinted at but did not resolve.

Each seed is a concrete, paste-into-a-search-engine string with a one-line note on the thread it follows and whether it is a DEPTH or BREADTH seed.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
## Round <!-- N --> — expansion seeds

<!-- One numbered seed per line, each a concrete search string with a note. Mix DEPTH and BREADTH seeds. -->

1. `<!-- concrete search string -->` — <!-- the thread it follows; DEPTH or BREADTH -->
2. `<!-- concrete search string -->` — <!-- the thread it follows; DEPTH or BREADTH -->
...

### Why these seeds

<!-- 2-3 lines: which promising threads the verified results opened that deserve a deeper pull, and which facets are still uncovered that the breadth seeds target. Reference specific results/sources the seeds spring from. -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Each seed is a concrete search string, not a topic description. Ground every seed in a specific result or source already found -- do not invent threads the research has not surfaced.
- Mix DEPTH and BREADTH. A round of pure depth seeds leaves breadth gaps; pure breadth seeds never reach the bottom of any thread.
- Seeds must be DIFFERENT from this round's planned queries and from prior rounds' -- expansion that re-queries the same strings wastes a round. If a facet needs re-querying, use a sharper, more specific phrasing.
- Do NOT search the web here. This step only writes the seeds; the next round's plan_queries consumes them.
- Append this block to log.md -- do not overwrite prior rounds' blocks.
- Execute only this instruction.
</rules>
