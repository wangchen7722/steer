---
parameter:
  instruction: string, required
  return: string, required
  check: string
  produce: list
on_check: |
  Verify the user's response:
  <check>{{ check }}</check>
  Review the reported answer and confirm it meets the condition.
---
Ask the user the following question and report their answer back.
<question>{{ instruction }}</question>
<method>
Use `AskUserQuestion` (or the host agent's equivalent) instead of just printing the question.

Choose the format:
- If the answer comes from a small, known set (yes/no, a/b/c, a list of IDs), provide 2–4 multiple-choice options. The user always has an "Other" choice for free input.
- If the answer is genuinely open-ended (free text, a path, a description), ask open-ended.

Even if you can infer a likely answer from context (e.g. the user already described the problem before starting this workflow), still call AskUserQuestion instead of filling in the answer yourself. Include your inferred answer as one of the multiple-choice options so the user can confirm it quickly or choose differently.
</method>
<rule>Never substitute your own answer. The value MUST come from the human user.</rule>
<report>Report the user's answer exactly as given via `steer instance set {{ steer_instance }} {{ steer_target }} <value>`, where <value> is the {{ return }}.</report>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}