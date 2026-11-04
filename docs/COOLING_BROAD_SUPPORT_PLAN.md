# Plan: broad motherboard fan support (FanControl approach)

Decision: DECISIONS.md #14. Goal: control fans on most motherboards on the
market, as FanControl does, through LibreHardwareMonitor (LHM) and PawnIO,
with behavioural safeguards where register readback is not available.

## How FanControl gets its coverage

FanControl does not ship per-board drivers. It uses LHM, whose SuperIO code
covers the chip families nearly all boards use (Nuvoton NCT67xx/NCT6687D,
ITE IT86xx/IT87xx, Fintek, Winbond), with PawnIO for register access. Coverage
is validated by its user base; a successful setter call is trusted. freeRGB
already loads LHM through PawnIO and a protected broker, but its production
gate (`NativeControl.CreateProtectedCore` -> `NativeNctEvidence.Create`) admits
17 Nuvoton chips with separate per-channel mode/command registers and audited
readback (NCT6771F ... NCT6799D, NCT6701D, NCT5585D, NCT610XD). Refused today:
ITE IT86xx/IT87xx (most Gigabyte/ASRock), Fintek, Winbond, Nuvoton NCT668x
(e.g. NCT6687D on many MSI boards) and embedded-controller readings.

LHM version (checked 2026-10-22): v0.9.6 (2026-02-14) is the latest stable
release, which freeRGB already pins. No upgrade is required now; chips newer
than 0.9.6 would need an unreleased LHM build and are out of scope.

## Support tiers

| Tier | Admission | Runtime protection |
|---|---|---|
| Verified | Audited register readback (today: Nuvoton separate-mode) | Register readback after every write (unchanged) |
| LHM-supported | Any LHM SuperIO `Control` with software mode, after the guided channel test below | RPM plausibility, telemetry freshness, temperature guard, fail-safe return |
| Read-only | Sensors without a usable control (e.g. many EC readings) | n/a |

The tier is shown in the GUI and the diagnostics report for every channel.

## Safeguards for the LHM-supported tier

1. **Guided channel test (once per channel, user-initiated):** record firmware
   RPM, set 100%, then 70% and 50% (never below 40%) for a few seconds each;
   require a monotonic, plausible RPM response and a successful return to
   firmware control. Channels with no tachometer, no response or an identified
   pump get no reduced duty (pump: 100% only). Result stored per channel
   identity (board, chip, control identifier).
2. **Runtime plausibility:** expected RPM band per duty from the test; repeated
   deviation, RPM 0 while commanded >= 40%, stale telemetry or temperature above
   the critical point -> return the channel to firmware (fallback: 100%) and
   mark it faulted until the user re-tests.
3. **Fail-safe release:** existing journal, watchdog and recovery path apply to
   every tier; release always restores the captured original mode.
4. **No silent foreign takeover:** refuse channels already in software mode
   (other tools), as today.

## Unrestricted unlock (owner decision, 2026-10-22)

Users can unlock **full control** of a channel on untested hardware, like
FanControl allows: any LHM `Control` with a software mode, without a passed
channel test and outside the verified/LHM-supported lists.

* Per channel, explicit and reversible: a dialog states the risks, the user
  types the channel name to confirm; stored with board, chip and control
  identity and invalidated when any of them changes (new BIOS, other board).
* Unlocked channels get tier `unrestricted`, shown everywhere (GUI badge,
  API, diagnostics, logs).
* What it lifts: the channel-test requirement, the 40% floor (0% / fan stop
  allowed), the RPM plausibility fault (it becomes a warning), chips without
  readback or known register scheme.
* What always stays, also when unlocked: return to the captured firmware mode
  on stop, daemon/service crash, lost telemetry and watchdog expiry; the
  critical-temperature override to 100%; refusal to take over a channel that
  is already in software mode (another tool owns it).
* Pumps: unlocking a channel marked as pump needs a second, separate
  confirmation; the 100% default stays until the user changes the curve.
* A global "unrestricted control" switch in settings (off by default) must be
  on before any per-channel unlock is offered.

## Work packages

1. **LHM version** *(implemented)*: stay on v0.9.6 (latest stable). The pin now
   exists only in `packaging/windows/mainboard-packages.json`; PowerShell
   (`Get-LhmPin`), C# (generated `LhmPin.g.cs`), Rust (`freergb_core::lhm` via
   `build.rs`) and Inno Setup (`/DLhmVersion`) derive it. A later upgrade is one
   change plus a re-audit of the private metadata used by `NativeControl` and
   `NativeNctEvidence`.
2. **Tiered native admission** *(implemented, synthetic tests only)*:
   `NativeNctEvidence` is optional. Missing register evidence
   (`NativeEvidenceUnavailableException`: non-SuperIO backend, non-Nct677X chip,
   unaudited Nct677X variant) admits the control as `lhm_supported` **only** when
   the protected authorization names a lower tier; every other evidence failure
   (vendor check, manual mode, ISA mutex) still refuses. Audited chips always keep
   register readback, also when unlocked. Legacy/verified authorizations, worker
   binding, journal entries and receipts are unchanged. Details:
   * Authorization/binding gain `tier` (`lhm_supported`|`unrestricted`), `role`
     (`fan`|`pump`) and `pump_unlocked`; `NativeAuthorizationRules` enforces
     40% fan / 100% pump, 0% and pump reduction only for `unrestricted`.
   * Journal entries gain `tier` and, without readback, recovery kind
     `lhm_fallback_v1` (service/worker lifetimes only). Crash recovery of a dead
     worker then drives the channel to a prepared 100% and holds it (reported as
     `held_full`) instead of "restoring" from a fresh LHM context, which would
     replay the crashed duty. The original firmware mode is only known inside
     the worker that captured it.
   * Lower-tier sessions return to the captured firmware mode inside the worker on
     any fault (100% first, then owned `SetDefault`, then journal clear) and
     report `tier`, `firmware_returned`, `warning`; verified sessions still hold
     100% for register-verified recovery.
   * Unrestricted telemetry treats a stalled fan as a warning; stale/invalid
     telemetry and the critical temperature still fault.
   * The snapshot reports `support` per control and the public SMBIOS `board`;
     `cooling.channels.list`, sessions, `diagnostics.get`/`export` and both GUIs
     show the tier. Rust runs lower tiers with `RuntimeLimits` and closes
     gracefully (return to firmware) on supervision faults instead of blocking.
3. **Multiple channels** *(implemented, synthetic tests only)*: journal version
   2 with one entry per channel, legacy v1 shape kept for 0/1 entries, liveness
   based admission, any-order release, per-channel crash recovery
   (`MAINBOARD_CONTROL_RECOVERY.md`). Interactive GUI sessions stay one at a
   time (single-instance bridge pipe); concurrent channels run in the protected
   background controller.
4. **Background control** *(implemented, synthetic tests only)*: the protected
   service (`BackgroundCooling.cs`) starts the saved per-channel curves 5 s after
   service start (boot), one protected worker per channel, without GUI or daemon.
   The curve evaluator mirrors `FanCurve`/`CurveController` (shared vectors in
   `crates/freergb-control/tests/fixtures/curve_vectors.json`); LHM-supported
   channels close (return to firmware) on persistent RPM implausibility and stay
   faulted until re-tested, unrestricted channels only warn. Worker faults retry
   after 30 s (10 min after three failures). Interactive sessions suspend the
   background curve of the same channel. Configuration only via the elevated,
   consented `--configure-cooling`; status in `mainboard-control-status.json`;
   API `cooling.background.*`; both GUIs edit and observe.
5. **Guided channel test + identification** *(implemented, synthetic tests only)*:
   `mode:"channel_test"` (100/70/50%, pumps excluded), results bound to
   board/BIOS/channel in `cooling-fan-support.json`, used for the 40% LHM floor
   and background RPM plausibility; pump marking via channel bindings; tier and
   test state in both GUIs. Runtime plausibility is enforced by the background
   controller; short interactive curve sessions rely on telemetry/stall guards.
   **Unrestricted unlock** (safeguard section above) follows as its own step.
6. **Diagnostics report** *(implemented)*: opt-in `cooling_hardware` section in
   `diagnostics.export` (board model, BIOS, chip, tier, test result), button in
   both GUIs; see `DIAGNOSTICS.md`.
7. **Modern GUI live control** *(implemented)*: curve, fixed duty, calibration,
   channel test, stop, tier/test badges, background and unlock sections match
   the Classic dialog.
8. **Remaining controller features** from `FANCONTROL_PARITY.md`: manual
   percentage *(implemented as `mode:"manual"`)* and step-up/step-down limits
   *(implemented: optional `max_rise_percent_per_s`/`max_fall_percent_per_s` on
   curves, Rust and C# checked against shared vectors; invalid sensors and
   critical heat still jump to 100% immediately)*; cooling profiles *(implemented: named
   background channel sets, re-validated on activation, both GUIs)*; still open:
   RPM targets, more curve types.
9. **Acceptance on the development board** for every package that touches
   hardware, with the user present; other boards via beta testers.

Order: 1 (small) -> 2 -> 3 -> 4 -> 5, then 6-8 in parallel where independent; 9
accompanies each hardware-facing step.

## Explicit risks accepted

* On chips without readback, a write that silently fails is detected only via
  RPM behaviour, i.e. after a delay, not immediately.
* Boards whose BIOS fights software control (e.g. some vendor "smart fan"
  firmware) may oscillate; the plausibility guard returns them to firmware.
* LHM chip support itself is community-tested; a wrong LHM mapping can address
  the wrong header. The guided test is the main protection.

### Implementation status (2026-10-22, branch feature/broad-fan-support)

Work packages 1-5 and the unrestricted unlock are implemented with synthetic
tests only (no hardware was opened). The unrestricted unlock: global switch
`cooling.support.settings` (default off), per-channel `cooling.channels.unlock`
with the typed channel name, `cooling.channels.confirm_pump` (`PUMP <name>`) for
pumps, `cooling.channels.revoke`; records bound to board/BIOS/channel; both GUIs.
Open: work packages 6-8 (opt-in diagnostics report with board/chip/test result,
modern-GUI live parity, further controller features) and hardware acceptance.
Known limits: interactive GUI sessions run one at a time (single-instance bridge
pipe); a dead lower-tier worker's channel is held at 100% until reboot and the
`held_full` marker is kept only in the snapshot worker's memory.

### Hardware acceptance checklist (with the user present)

Prerequisites: build the installer from this branch, install with the mainboard
option, note the current BIOS version, keep LHM/FanControl/vendor tools closed.

1. **Version pin:** `freergbd --help` and Setup show LHM 0.9.6; `%ProgramFiles%\freeRGB\providers\lhm-0.9.6` exists.
2. **Inventory:** `cooling.channels.list` shows `support.tier` per control (NCT6701D: `verified`), `software_mode=false`; `cooling.providers.list` shows `board`.
3. **Verified unchanged:** on CHA_FAN2 run calibration and an 80% curve (UAC); report `tier:null|verified`, `return_evidence:register_readback`; journal file is `{"version":1,"reservation":null}` afterwards.
4. **Foreign takeover refused:** start FanControl on one header, then freeRGB on it -> refused with the software-mode message; close FanControl.
5. **Channel test:** run `channel_test` on CHA_FAN2; observe 100/70/50%, RPM falls monotonically, fan returns to firmware; `cooling.support.get` shows `passed` with RPM values.
6. **Fault return:** during a curve, unplug the tachometer or stop the daemon -> 100%, then firmware mode; no `blocked` state for lower tiers; journal cleared.
7. **Multiple channels:** add two fans to background control (UAC), reboot without logging into the GUI; both follow their curves (status file), journal shows a version-2 file while both run.
8. **Interactive precedence:** start a live session on a background channel -> background suspends, resumes after stop.
9. **Crash recovery:** kill one background worker process (Task Manager, admin) -> verified chip: firmware mode restored; untested chip: channel held at 100% (`held_full`) until reboot.
10. **Plausibility:** on an LHM-supported board (e.g. ITE), block a fan briefly (safely) during background control -> channel returns to firmware and shows `rpm_implausible`.
11. **Unrestricted:** enable the switch, unlock a fan by typing its name, run a curve with 0% minimum -> fan stops, warning only; revoke -> background re-validated (UAC), channel back to >=40% or removed.
12. **Pump:** bind the AIO pump header as pump; unlock without pump confirmation -> stays 100%; after `PUMP <name>` a reduced curve is accepted (only test briefly, watch coolant temperature).
13. **Identity binding:** after a BIOS update the test and unlock show `stale`.
14. **Maintenance:** run Setup upgrade while background control runs -> channels return to firmware, upgrade succeeds, background restarts.

### Coverage matrix and chip-family simulation (2026-11-04)

Support is measured per fan-controller chip in the generated
[FAN_COVERAGE.md](FAN_COVERAGE.md) (`scripts/cooling/fan-coverage.mjs`, CI
`fan-coverage-drift.yml`). Every LHM chip with controls belongs to a simulator
family in `crates/freergb-control/tests/fixtures/chip_families.json`;
`tests/chip_families.rs` runs the guided channel test, RPM plausibility
(`freergb_control::plausibility`, mirror of the C# `RpmPlausibility` with shared
vectors), fault return and unrestricted unlock against register-level fakes
with the family quirks (shared mode registers, 7-bit PWM, clamped duty range,
ignored writes, delayed RPM, missing readback). Linux `pwmN_enable` semantics
per driver are a table in `hwmon.rs` with simulated sysfs tests. Simulation is
not hardware evidence: only NCT6701D is hardware-verified, and only for the
CHA_FAN2 scope in COOLING_ACCEPTANCE.md.
