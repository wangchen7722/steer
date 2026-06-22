---
parameter:
  instruction: string, required
  return: string, required, default="output"
  produce: list
  check: string
on_check: |
  Verify the command result:
  <check>{{ check }}</check>
  Check the command output, exit code, or resulting files.
---
Run the following shell command and capture its output (stdout, stderr, exit code).
<command>{{ instruction }}</command>
<report>Report the output via `steer instance set {{ steer_instance }} {{ steer_target }} <value>`, where <value> is the {{ return }}. If the command fails, report the failure honestly instead of fabricating success.</report>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}<rule>Run the command exactly as given. Do not modify flags or add extra steps.</rule>
