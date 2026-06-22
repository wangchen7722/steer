---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the following:
  <check>{{ check }}</check>
  Inspect the work and confirm the condition holds.
---
Follow the instruction below{% if return %} and report back{% endif %}.
<instruction>{{ instruction }}</instruction>
{% if return %}<report>Report the result via `steer instance set {{ steer_instance }} {{ steer_target }} <value>`, where <value> is the {{ return }}. If you cannot complete the work, use `steer instance error {{ steer_instance }} "<reason>"` instead of guessing.</report>
{% endif %}{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}<rule>Execute only this instruction. Do NOT skip ahead or do unplanned work.</rule>
