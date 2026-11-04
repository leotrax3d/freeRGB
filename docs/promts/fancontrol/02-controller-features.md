# Prompt 02 — Remaining controller, curve and calibration features

Read `00-common-rules.md` first. Branch: `cooling/controller-features`.
Depends on prompt 01 (use its simulators for tests).

## Goal
Close every "Missing"/"Preview" row in the Control, Curves and Calibration
sections of `docs/FANCONTROL_PARITY.md`.

## Tasks
Extend `crates/freergb-control` (pure policy) and the daemon live sessions;
reuse existing `curves.rs`, step limits and composite curves.
1. **Calibration:** finer duty steps; detect start (spin-up) and stall
   thresholds per channel; store with channel identity; never below safety floor
   unless tier `unrestricted`; never on pumps.
2. **Resonance avoidance:** user-defined forbidden duty/RPM ranges, controller
   skips through them deterministically.
3. **Startup/stop behavior:** spin-up boost when leaving 0 % or the stall zone;
   optional fan stop below a threshold only where tier allows; pump-safe.
4. **Delay & hysteresis:** response delay (s), up/down hysteresis, boundary
   behavior exactly defined and unit-tested.
5. **Duty offset / per-device min/max** on top of any curve.
6. **Closed-loop RPM targets:** PI correction on top of the open-loop
   calibration map, anti-windup, falls back to open-loop on bad telemetry.
7. **Graph curve:** arbitrary point count, linear/step interpolation;
   **linear** and **graph** natively bound to channels.
8. **Adaptive equilibrium curve:** trend-based controller (target temperature,
   max change rate) as a new composite node; cycle-checked, fail-safe like others.
9. Manual percentage per channel as a first-class mode (supervised).
10. API (`cooling.*`) and storage for all of the above, documented in
    `docs/LOCAL_API.md`; Tauri editor support for new parameters (minimal, the
    full UX is prompt 05).

## Done when
Every feature is a pure, unit-tested policy plus daemon integration tested
against the chip simulators; missing/stale sensor -> 100 % everywhere;
FANCONTROL_PARITY rows updated to "implemented (simulated)" with owner test
steps written down for prompt 07.
