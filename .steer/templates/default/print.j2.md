---
parameter:
  instruction: string, required
  return: none
  check: string
  produce: list
on_check: |
  Verify the output:
  <check>{{ check }}</check>
---
{{ instruction }}
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
