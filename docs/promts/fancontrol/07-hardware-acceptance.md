# Prompt 07 — Hardware acceptance on the owner board (owner present)

Read `00-common-rules.md` first. Branch: `cooling/acceptance-<date>`.
Run once all of prompts 01–06 are merged (05 adds GUI steps when it lands).

Hardware: ASUS PRIME X870-P WIFI, Nuvoton NCT6701D; case fan on CHA_FAN2
(`/lpc/nct6701d/0/control/2`, RPM `/lpc/nct6701d/0/fan/2`). The second
motherboard tachometer belongs to the Corsair iCUE LINK AIO pump: **never
reduce it**. GPU: owner NVIDIA card (NVML).

## Rules
- Ask the owner before every step that writes to hardware, stops the running
  `freergbd`, reboots, or needs admin rights; say what happens and how it is undone.
- Preconditions for every write step: CPU below 60 °C, stop-all ready
  (`freergb cooling stop-all`), iCUE closed for LINK steps.
- Record each result (date, build commit, step id, observation, pass/fail) in
  `docs/COOLING_ACCEPTANCE.md`. Failures are recorded too, with a fix task.
- Only recorded results may set `hardware-verified` in `docs/FAN_COVERAGE.md`
  and check boxes in `docs/STATUS.md` (regenerate the README progress block).
- Stop at the first safety-relevant failure (fan not returning to firmware/100 %,
  pump reduced, write after stop) and report before continuing.

## Session plan (in this order; detailed steps live in the linked docs)

### A. Baseline and discovery (~10 min, read-only)
A1. Build from `main`, note the commit. Start daemon, export the redacted
    hardware report (prompt 01); confirm board/chip/tier, CHA_FAN2 tier
    `verified`, pump channel identified and excluded from reduction.
A2. Add the report as the first evidence row in `docs/FAN_COVERAGE.md`.

### B. Controller and calibration (~40 min) — [FANCONTROL_PARITY.md](../../FANCONTROL_PARITY.md) "Owner hardware steps"
B1 fine sweep · B2 stall sweep (unrestricted unlock, pump refusal) ·
B3 closed-loop RPM ±3 % and stale-RPM fallback · B4 shaping (offset, max, delay,
hysteresis, avoided range) · B5 spin-up / fan stop (refusal on locked channel and
pump) · B6 linear, step and equilibrium curves incl. sensor-loss fault return ·
B7 daemon restart resumes only with unchanged identity.

### C. Sensors and GPU fans (~20 min) — [COOLING_SOURCES_GPU.md](../../COOLING_SOURCES_GPU.md)
C1. Every source class appears with plausible values: CPU package/CCD, GPU
    core/hotspot/memory, board, NVMe, LINK liquid temperature.
C2. Text-file sensor: write a number to a file, use it in a curve; stop
    updating the file -> stale -> curve goes 100 %.
C3. GPU fan (admin): take control, set 40 % and 70 %, compare RPM with the
    NVIDIA app; stop -> driver auto restored; kill the daemon during control ->
    driver auto restored (check with the NVIDIA app / `nvidia-smi` as admin).

### D. Service, profiles, CLI, lifecycle (~45 min, reboots) — [COOLING_SERVICE.md](../../COOLING_SERVICE.md) "Owner hardware steps"
D1 login-mode baseline · D2 service mode boot without login · D3 profile
switching by CLI/tray/shortcut/app rule · D4 export/delete/import · D5 watchdog
via suspended daemon · D6 upgrade / rollback / uninstall during active control.

### E. Cooling plugins (~20 min) — [COOLING_PLUGINS.md](../../COOLING_PLUGINS.md) "Owner hardware steps"
E1 build/sign/install the iCUE LINK plugin · E2 readings match `link-probe`/iCUE ·
E3 hub returns to hardware mode after each sample · E4 kill plugin -> failsafe ·
E5 unplug/replug the hub.

### F. GUI walkthrough (after prompt 05 is merged)
Guided setup on CHA_FAN2 end-to-end, dashboard live values, curve editors, tray
profile switch and fault notification; steps from prompt 05's report.

### G. Endurance (~2 h, unattended after start)
G1. Composite curve on CHA_FAN2 + GPU fan under a varying load script for 2 h;
    afterwards: no stuck channels, no drift beyond calibration, journal clean.

### R. Re-test after prompt 08 (acceptance fixes, branch `cooling/fix-reader-hang`)
Install the build from the merged PR over the current install (one UAC).
R1. Installer single instance: start the setup twice quickly; the second must
    say "Setup is already running" while the first waits behind UAC.
R2. Broker pipe after upgrade (A1 re-test): right after setup, without
    `Restart-Service`, `freergb.exe ctl cooling.providers.list` shows
    `mainboard.lhm` `ready` within 10 s (no `ERROR_PIPE_BUSY`, no permanent
    `permission_denied`/`starting`).
R3. Roles: open guided setup; every header with RPM shows a role or
    "100 % (role unknown)"; Calibrate/Apply stay disabled until all have one.
    Bind the AIO pump (`control/1`) as `pump`.
R4. Job origin: `cooling.sessions.status` before any request; every listed job
    has `origin` (`resume`/`profile`/`api`). Note any unexpected `api` jobs
    with the desktop app closed vs. open.
R5. Fine sweep (B1 re-test): `cooling.sessions.start`
    `{"mode":"calibration","sweep":{"step_percent":5,"lowest_percent":40},...}`
    on CHA_FAN2 (`control/2`, rpm `fan/2`). Expect 13 `tested` steps,
    `report.calibration` set, `cooling-fan-support.json` written,
    `rpm_range` in `cooling.support.get`. If it ends early, record
    `report.fault` ("Calibration aborted: <reason>") — the reason is now shown.
R6. Plain calibration (100/80/60/40) on the same channel stores its result.

## Done when
All steps recorded with commit and result; failures turned into fix tasks;
STATUS/FAN_COVERAGE/README progress updated only from recorded evidence.
