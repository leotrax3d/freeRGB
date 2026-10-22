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
gate (`NativeControl.CreateProtectedCore` -> `NativeNctEvidence.Create`) only
admits Nuvoton chips with separate mode registers and audited readback.

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

## Work packages

1. **LHM upgrade** to the current release for newer chips; re-audit the private
   metadata used by the zero-duty workaround and `NativeNctEvidence`, rerun the
   synthetic callback tests. (Prerequisite for coverage.)
2. **Tiered native admission:** `NativeNctEvidence` becomes optional; without
   it the adapter is created as LHM-supported with behavioural evidence
   supplied by the Rust runtime. Keep mode-readback checks LHM exposes.
3. **Multiple channels:** extend the durable native journal and the session
   host from one channel to all reserved channels (per-channel recovery
   records, release in any order, crash recovery for all).
4. **Background control:** the protected service starts the saved per-channel
   curves at boot/login without a GUI session; GUI only edits and observes.
5. **Guided channel test + identification** in the service API and both GUIs
   (which header is which fan, pump marking, tier display).
6. **Diagnostics report** (opt-in, anonymised: board model, chip, tier, test
   result) so LHM-supported chips can be promoted with evidence.
7. **Modern GUI live control** (parity with the Classic dialog).
8. **Remaining controller features** from `FANCONTROL_PARITY.md` (manual
   percentage, RPM targets, rise/fall limits, more curve types, profiles).
9. **Acceptance on the development board** for every package that touches
   hardware, with the user present; other boards via beta testers.

Order: 1 -> 2 -> 3 -> 4 -> 5, then 6-8 in parallel where independent; 9
accompanies each hardware-facing step.

## Explicit risks accepted

* On chips without readback, a write that silently fails is detected only via
  RPM behaviour, i.e. after a delay, not immediately.
* Boards whose BIOS fights software control (e.g. some vendor "smart fan"
  firmware) may oscillate; the plausibility guard returns them to firmware.
* LHM chip support itself is community-tested; a wrong LHM mapping can address
  the wrong header. The guided test is the main protection.
