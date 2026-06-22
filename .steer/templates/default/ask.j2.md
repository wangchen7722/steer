---
formatter:
  instruction: string, required
  return: string
  check: string
---
<ask>
  <instruction>{{ instruction }}</instruction>
  <method>Ask via your `AskUserQuestion` tool (or the host agent's equivalent structured-question tool). Prefer 2–4 multiple-choice options when the answer is a small, known set — the user always has an "Other" choice for free input; if it is genuinely open-ended (an ID, a path, free text), ask open-ended through the same tool. Do not just print the question and stop to wait: call the tool.</method>
{% if return %}  <report>After the user answers, report it with `steer instance set {{ instance }} {{ target }} <value>`, in this format: {{ return }}</report>
{% endif %}</ask>
