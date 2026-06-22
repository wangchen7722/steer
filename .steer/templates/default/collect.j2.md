---
parameter:
  instruction: string, required
  return: string, required, default="result"
  check: string
  produce: list
on_check: |
  Verify the finding:
  <check>{{ check }}</check>
  Review the reported result and confirm it meets the condition.
---
Investigate the following on your own and report the finding.
<instruction>{{ instruction }}</instruction>
<rule>Do the actual work yourself: read files, trace behavior, reason it through. Ground your answer in what you examined. Do not guess or hand-wave.</rule>
<report>Report the result via `steer instance set {{ steer_instance }} {{ steer_target }} <value>`, where <value> is the {{ return }}. If you cannot reach a confident answer, use `steer instance error {{ steer_instance }} "<reason>"` instead of fabricating.</report>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
