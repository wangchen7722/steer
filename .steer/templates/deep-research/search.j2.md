---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the harvested results:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **search** -- fan out parallel web-search subagents, one per planned query, then merge their results.

Read the `## Round {round} -- planned queries` block in `log.md`. For each planned query, dispatch a parallel subagent that performs a web search with that exact keyword/phrase and collects results. Run the subagents concurrently (they are independent) -- do not search them one at a time.

Each search subagent:
1. Runs the web search with its assigned keyword/phrase.
2. Collects the top results (title, URL, snippet, and publish date if visible on the result page).
3. Returns its result list to the main agent.

The main agent then merges ALL subagents' results into `sources.md` as a single `## Round {round} -- raw results` block. Preserve each result's **query origin** (which planned query surfaced it) -- that traceability matters for cross-validation later. This step only HARVESTS; it does not fetch full pages, verify, or dedup -- that is the next step.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
## Round <!-- N --> — raw results

<!-- One sub-block per planned query. Under each, list the results that query surfaced. -->

### Query 1: `<!-- the search string -->`

- **[<!-- title -->]**(<!-- URL -->) — <!-- publish date if visible, else "date unknown" --> — <!-- one-line snippet -->
- ...

### Query 2: `<!-- the search string -->`

- **[<!-- title -->]**(<!-- URL -->) — <!-- publish date if visible, else "date unknown" --> — <!-- one-line snippet -->
- ...

<!-- ...one sub-block per planned query... -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Fan out one subagent per planned query, in parallel. Do not serialize independent searches.
- Each result entry records title, URL, snippet, publish date (if visible; "date unknown" otherwise), and the query that surfaced it. Query origin is mandatory -- it is how cross-validation traces a claim back to its search.
- Harvest only. Do NOT fetch full pages, do NOT assess credibility, do NOT dedup here -- that is the process step's job.
- If a subagent's search returns nothing useful, record that explicitly ("no relevant results") rather than omitting the query -- an empty query is still information.
- Append this block to sources.md -- do not overwrite prior rounds' blocks.
- Execute only this instruction.
</rules>
