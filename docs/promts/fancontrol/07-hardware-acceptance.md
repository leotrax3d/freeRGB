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
C4. AMD GPU (RX 9060 XT, ADLX; prompt 09): `cooling.gpu.list` shows
    `adlx:768/fan0` controllable, minimum 15 %; the iGPU is listed as not
    controllable. Take control, set 40 % and 70 %, compare RPM/fan curve with
    AMD Software (Performance -> Tuning); stop -> the previous fan curve and
    zero-RPM setting are back. Kill the daemon during control, restart ->
    the default fan curve is applied (check AMD Software). Record whether
    writes need administrator rights.
C5. Drive temperatures: `monitoring.sensors.list` shows provider `storage`
    for the NVMe/SATA drives (990 PRO x2, Toshiba HDD) with plausible values;
    motherboard fan readings keep updating every second meanwhile. Use one
    drive temperature in a curve.

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
R5c. Loaded native startup/acknowledgement re-test (owner present): on the
    confirmed CHA_FAN2 fan, repeat R5 at idle, then with four concurrent Cargo
    builds (record Tctl and CPU load). Record time to the first active receipt,
    session status and any broker stage/error. Expect one acquisition, startup
    within 30 s and no immediate acknowledgement block; complete all 13 steps.
    Stop explicitly and verify firmware return plus a clear journal. Repeat a
    start/stop under load. A genuine command/ticket/lease timeout must still
    stop safely and retain recovery intent until authenticated return evidence;
    never restart a blocked session automatically. Record commit and outcome in
    COOLING_ACCEPTANCE.md. Synthetic silent-worker tests cover timeout behavior;
    do not suspend/kill a real worker during this calibration re-test.
R6. Plain calibration (100/80/60/40) on the same channel stores its result.
R7. Reader stability (item 8): with iCUE running, sample
    `freergb.exe ctl cooling.providers.list` every 5 s for 2 min; expect
    `mainboard.lhm` ready in (almost) every sample and no "reader failed"
    loop. Note any "Slow sensor updates" hint in the message (which hardware,
    how many ms). Then repeat R5.
R8. Maintenance drain with a background curve (item 9 / D6): with the
    CHA_FAN2 background curve running, run the next installer; setup must
    not fail with "Mainboard-Rückgabe nicht bestätigt"; afterwards the curve
    runs again (service start).
R9. Bindings (item 10): `cooling.bindings.get` after one save from the
    desktop shows one CHA_FAN2 binding (channel-id spelling) and the pump
    binding; assigning CHA_FAN2 again in guided setup replaces it.

## Done when
All steps recorded with commit and result; failures turned into fix tasks;
STATUS/FAN_COVERAGE/README progress updated only from recorded evidence.


## Release-SHA acceptance appendix (proposed public beta)

Planning only; no results are implied. Follow the existing owner-present rules and
`00-common-rules.md`. No hardware, elevation, service changes or installer execution
is authorized by this appendix itself. The release coordinator supplies the full
candidate SHA, installer filename and SHA-256 from `docs/legal/RELEASE_SCOPE.md`.

- [ ] Record OS/build, board/chip, firmware, provider/module/driver versions,
  device/channel identity and role, candidate SHA, exact package hash, date and
  owner/operator for each result in `docs/COOLING_ACCEPTANCE.md`.
- [ ] Select applicable A-G and R steps for each operation proposed for release;
  mark unavailable platforms/devices as unexecuted and exclude unsupported claims.
  Preserve pump restrictions and stop criteria; do not infer other channels' results.
- [ ] Record acquisition, calibration, explicit stop/stop-all, sensor loss, session
  revocation, restart and restoration observations for each included backend.
  Fault/watchdog and disconnect steps need their own owner authorization under the
  existing rules; record actual restoration observations and any incomplete return.
- [ ] With separate owner authorization for installer execution/elevation, cover
  fresh install, upgrade, rollback, uninstall and reboot/service lifecycle using
  the exact hashed candidate. Check active-control drain and resulting channel
  state, as well as installed notices/configuration and offline defaults.
- [ ] Check UI limitations and hardware matrix against the recorded results;
  capture failures, recovery actions, exclusions and follow-up tasks. A different
  build or payload requires re-evaluation of affected steps.
- [ ] Link evidence to the release-SHA record. Owner records scope acceptance or
  postponement; missing evidence stays open. This appendix grants no legal approval.

## S21: Gigabyte Fusion 2 USB variants (pending owner hardware acceptance)

Software fixtures only; no hardware or real bus access performed. Test each of
`048d:5711`, `048d:8950`, `0414:a100` when the owner has matching hardware.
Record board/controller identity, firmware, commit, selected channel, configured
LED count and outcome. Keep vendor RGB software closed during acceptance.

- Confirm independent red/green/blue output and LED addressing on each configured
  channel: IT5711 four external headers plus two onboard channels, 8950 two
  headers, A100 one header. Leave unavailable channels unconfigured.
- Check packet boundaries with 19/20 LEDs and strip class transitions at 32/33
  and 64/65 LEDs; IT5711 must reject counts above 256 before any HID write.
- Confirm unused channels retain firmware effects and length classes. For IT5711,
  select header 3 or 4 independently to verify CC61 calibration and sparse mode bits.
- Disconnect and close the app: confirm stored firmware effects return. No flash
  save, calibration programming, driver installation or cooling commands are part
  of this acceptance. Software support remains Experimental until recorded here.
