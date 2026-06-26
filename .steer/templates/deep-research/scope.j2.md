---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the scope:
  <check>{{ check }}</check>
---
This is the deep-research workflow. Current position: **scope** -- refine the research question into a sharpened brief before any searching begins.

The research has not started yet. Do NOT search the web here. Your job is to map the question's surface area so later rounds know what breadth and depth to target. On a refinement pass (when `scope.md` already exists and the user gave feedback), read the existing brief FIRST and merge the feedback in -- sharpen the question, add/remove/refocus facets, adjust depth targets -- rather than starting over.

Read the question carefully, then enumerate the distinct facets/aspects, name a depth target for each, record any constraints (time frame, region, audience, authority required), and write a testable success criterion. THEN propose 2-3 concrete directions the user could pick to deepen or extend the research -- these become the menu for the user's next decision.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Use the following as your output template. Follow this structure exactly, replacing each `<!-- ... -->` placeholder with real content and removing the placeholder comments from the final file.

<template>
# Research Scope — <!-- run name -->

## Question

<!-- The refined research question. On a refinement pass, sharpen it from the prior brief plus the user's feedback; on the first pass, restate the original question with a one-line clarification of what a satisfying answer must establish. -->

## Facets to cover (breadth)

<!-- List every distinct facet/aspect a thorough answer must address. One bullet per facet with a short name + one line on what it covers and why it matters to the question. On a refinement pass, add/remove/refocus facets per the user's feedback. Aim for completeness over neatness -- an omitted facet is a blind spot later rounds will not recover. -->

- **<!-- facet name -->**: <!-- what this facet covers and why it matters -->
- ...

## Depth target per facet

<!-- For each facet above, state how deep the research should go: e.g. "definitions + current best practice + 2-3 named examples", "mechanism + trade-offs + quantitative benchmarks", "timeline + key actors + outcomes". Depth targets guide how many rounds each facet deserves. -->

## Constraints

- **Time frame**: <!-- e.g. "2023-present", "no constraint", "historical only" -->
- **Authority**: <!-- e.g. "prefer primary sources / official docs / peer-reviewed", "vendor docs acceptable for tooling" -->
- **Region / audience / scope limits**: <!-- any other constraints inferred from the question -->
- **Out of scope**: <!-- explicitly name what is NOT being researched, to keep later rounds focused -->

## Success criterion

<!-- State, in one or two lines, what "covered" means for this question -- i.e. what evidence across which facets would make the answer thorough enough to synthesize. The research loop's judge step measures against this. -->

## Suggestions for further refinement

<!-- 2-3 concrete directions the user could pick to deepen or extend the research. Each must be a SPECIFIC angle, subtopic, comparison, or reframing the user could adopt -- never a vague "research more" or "be more thorough". The user will be offered these as a menu: they can APPROVE to start, or pick one / give their own clarification to fold back in. -->

1. <!-- a specific deepening or extension direction, with one line on what it would add -->
2. <!-- a specific deepening or extension direction, with one line on what it would add -->
3. <!-- a specific deepening or extension direction, with one line on what it would add -->
</template>

<rules>
- LANGUAGE: Write all output in English.
- Do NOT search the web in this step. Scope is analysis of the question, not research.
- On a refinement pass, read the existing scope.md FIRST and merge the user's feedback in. Sharpen the brief; do not discard prior refinement and start over.
- Enumerate facets exhaustively. A facet missed here is a facet the research will never cover.
- Keep facets distinct (non-overlapping); if two overlap, merge or split them so each is a single aspect.
- The success criterion must be testable: the judge step will later ask "are all important facets covered with verified, sufficient evidence?" -- write the criterion so that question has a clear answer.
- The "Suggestions for further refinement" are a decision menu for the user, not a to-do list for the research. Each must be concrete and pickable. Do NOT write more than 3, and do NOT repeat facets already in the brief.
- Execute only this instruction. Do NOT skip ahead to searching.
</rules>
