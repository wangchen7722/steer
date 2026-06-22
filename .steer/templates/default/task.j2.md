---
formatter:
  instruction: string, required
  return: string
  check: string
  produce: list
---
<task>
  <instruction>{{ instruction }}</instruction>
{% if return %}  <report>Report the result with `steer instance set {{ instance }} {{ target }} <value>`, in this format: {{ return }}</report>
{% endif %}{% if produce %}  <produce>{% for f in produce %}<file>{{ f }}</file>{% endfor %}</produce>
{% endif %}</task>
