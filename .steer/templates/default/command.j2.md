---
formatter:
  instruction: string, required
  return: string
  produce: list
  check: string
---
<command>
  <instruction>{{ instruction }}</instruction>
{% if return %}  <report>Report the output with `steer instance set {{ instance }} {{ target }} <value>`, in this format: {{ return }}</report>
{% endif %}{% if produce %}  <produce>{% for f in produce %}<file>{{ f }}</file>{% endfor %}</produce>
{% endif %}</command>
