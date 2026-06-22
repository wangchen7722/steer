---
parameter:
  instruction: string, required
  return: bool
  check: string
  produce: list
on_check: |
  Verify the judgment:
  <check>{{ check }}</check>
  Confirm the boolean answer is correct.
---
Evaluate the following and answer true or false.
<question>{{ instruction }}</question>
<answer>Set your answer via `steer instance set {{ steer_instance }} {{ steer_target }} true` (or `false`). Answer only `true` or `false`, no explanation or hedging. If you genuinely cannot determine, use `steer instance error {{ steer_instance }} "cannot determine"`.</answer>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
