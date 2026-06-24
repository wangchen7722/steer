---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the full structural validation:
  <check>{{ check }}</check>
---
This is the openspec-generate-specs workflow. Current position: **full-validate** -- the one whole-repo `openspec validate --specs` pass in the run. generate_all validated each spec singly; refine may have edited or created specs since, so this catches anything cross-spec before reporting success. This step runs commands and fixes specs; it does NOT produce a file.

For each target repo that received specs (read the list from generation-log.md): run `cd <repo> && openspec validate --specs`, read its exit code AND output. Fan out one subagent per repo -- each repo validates independently (parallel), and within a repo fixes are serial (they touch that repo's own specs, so two subagents never edit the same repo). In a multi-repo checkout there is no root `openspec/` -- run from each repo's cwd, never the manifest root. If it exits non-zero or prints any `[ERROR]` line, fix the offending spec(s) in that repo (same fixes as the per-cap validate: SHALL/MUST keyword, exactly-four-hashtag scenarios, Purpose >= 50 chars) and re-run. Do not report success until every touched repo exits 0 with no ERROR lines.

<instruction>{{ instruction }}</instruction>
<rules>
- LANGUAGE: English output.
- Validate PER target repo that received specs (from generation-log.md), from that repo's cwd -- never the manifest root in a multi-repo checkout.
- A non-zero exit, or exit 0 with any `[ERROR]` line, is a failure -- fix and re-run until exit 0 with no ERROR lines, in every touched repo.
- Execute only this instruction.
</rules>
