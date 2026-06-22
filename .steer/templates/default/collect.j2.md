---
formatter:
  instruction: string, required
  return: string
  check: string
---
<collect>
  <instruction>{{ instruction }}</instruction>
  <method>This is a reasoning op. The value must come from YOUR OWN investigation and analysis — not from asking the user (that is `ask`) or running a single shell command (that is `command`). Actually do the work the instruction describes: read the relevant files or code, trace or reproduce the behavior, reason it through. Then report the value that work produces, grounded in what you examined — do not guess or hand-wave.</method>
{% if return %}  <report>Report the result with `steer instance set {{ instance }} {{ target }} <value>`, in this format: {{ return }}</report>
{% endif %}</collect>
