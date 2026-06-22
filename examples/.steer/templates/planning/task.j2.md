---
formatter:
  instruction: string, required
  return: string
  produce: list
  check: string
---
PLANNING MODE

Prepare planning material before implementation:
{{ instruction }}
{% if return %}- Report the planning result via `steer set {{ target }}` in this format: {{ return }}
{% endif %}{% if produce %}- Planning artifacts: {% for f in produce %}{{ f }} {% endfor %}
{% endif %}
