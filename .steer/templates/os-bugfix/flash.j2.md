---
parameter:
  instruction: string, required
  return: string
  check: string
  produce: list
on_check: |
  Verify the flash:
  <check>{{ check }}</check>
  Confirm the device was flashed and rebooted to normal mode with both human gates answered yes.
---
Current position: **flash** (human-in-the-loop, real device). Read the flash command and device profile from the current bug's flash.md. Do NOT skip the human gates.

Gate sequence (use AskUserQuestion for each):
1. Ask: is the device in flash/download mode and connected (USB)? If NO, report this check FAILED with reason `device not ready` and stop — the step will retry and re-ask.
2. Run the flash command exactly as recorded in flash.md.
3. Ask: did the device flash successfully and reboot to normal mode? If NO, report this check FAILED with the observed error.

Only when both gates are YES do you pass the check.

<instruction>{{ instruction }}</instruction>

<rules>
- LANGUAGE: English output.
- Never flash without the human confirming readiness.
- Run the flash command exactly as recorded; do not modify flags.
- If a gate fails, fail the check with a concrete reason — do not silently retry forever within one attempt.
- Execute only this instruction.
</rules>
