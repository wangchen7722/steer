---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the captured commands:
  <check>{{ check }}</check>
  Confirm device, build, flash, and verify commands are all present and user-sourced.
---
Current position: **capture toolchain commands**. You MUST obtain the target device, the build command, the flash command, and the verify command FROM THE USER via AskUserQuestion. Do NOT invent or guess any command — every command must come from the human. These are cached once and reused every iteration and every resumed run, so get them right.

This is a Rust + GN/Ninja OS codebase on a real device. Typical (NOT prescriptive — confirm with the user) shapes: a GN+Ninja build for a product target; a vendor flash tool or fastboot-like flow to a connected device; an on-device or host-driven verification. Record the user's real commands verbatim.

<instruction>{{ instruction }}</instruction>
{% if produce %}<produce>Write or update the following files as part of this work:{% for f in produce %}
- {{ f }}{% endfor %}
</produce>
{% endif %}
Write flash.md and verify.md using these templates.

<template>
# Device profile & build/flash commands: <!-- slug -->

- **Target device / board / product**: <!-- from user -->
- **Build command**: `<!-- exact shell command from user -->`
- **Flash command**: `<!-- exact shell command from user -->`
- **Image / artifact flashed**: <!-- what the flash command pushes -->
- **Human prep needed before flash**: <!-- e.g. enter download mode, connect USB -->

## Notes

<!-- anything the user added about ordering, env vars, or paths -->
</template>

<template>
# Verify command: <!-- slug -->

- **Verify command**: `<!-- exact shell command from user, or 'human-on-device' if the host cannot drive it -->`
- **What success looks like**: <!-- pass criterion the user defines -->
</template>

<rules>
- LANGUAGE: English output.
- EVERY command value MUST come from the user via AskUserQuestion. Never fabricate.
- If the user cannot provide one yet, leave a clear TODO and say so — do not guess.
- Execute only this instruction.
</rules>
