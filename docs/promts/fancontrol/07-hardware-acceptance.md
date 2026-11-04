# Prompt 07 — Hardware acceptance on the owner board (owner present)

Read `00-common-rules.md` first. Branch: `cooling/acceptance-<date>`.
Run after each of prompts 01–06 for the steps they listed, and once fully at
the end.

Hardware: ASUS PRIME X870-P WIFI, Nuvoton NCT6701D; case fan on CHA_FAN2
(`/lpc/nct6701d/0/control/2`). The second motherboard tachometer belongs to the
Corsair iCUE LINK AIO pump: **never reduce it**.

## Rules
- Ask the owner before every step that writes to hardware or stops the running
  `freergbd`; state what will happen and how it is undone.
- Record each result (date, build commit, step, observation, pass/fail) in
  `docs/COOLING_ACCEPTANCE.md`. Failures are recorded too, with a fix issue.
- Only confirmed results may set `hardware-verified` in `docs/FAN_COVERAGE.md`
  and check boxes in `docs/STATUS.md`.

## Checklist (extend with steps from earlier prompts)
1. Discovery: all headers, correct tier, pump identified.
2. Guided channel test and calibration incl. start/stall thresholds.
3. Each curve type and controller feature from prompt 02 on CHA_FAN2, observed
   RPM and noise; resonance skip; closed-loop RPM target accuracy.
4. Sensors from prompt 03 incl. text-file sensor and GPU fan take/release.
5. Fault return: stop daemon, kill worker, stale sensor, unplug tachometer ->
   100 %, then firmware mode.
6. Boot-time control after reboot without login; profile switch by app rule;
   CLI commands; upgrade, uninstall and rollback during active control.
7. Sustained 2-hour run with variable load; no drift, no stuck channels.
8. Export the community hardware report for this board and add it as the first
   matrix evidence.

## Done when
All steps recorded; failures turned into fix tasks; STATUS/FAN_COVERAGE updated
only from recorded evidence.
