---
formatter:
  instruction: string, required
  return: bool
---
<judge>
  <instruction>{{ instruction }}</instruction>
  <answer>Answer with only `true` or `false`, set via `steer instance set {{ instance }} {{ target }} true` (or `false`).</answer>
</judge>
