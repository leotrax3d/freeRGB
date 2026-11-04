# FanControl functionality target

The user clarified that the decisive FanControl requirement is broad support for
users' motherboard-connected fans, alongside separate motherboard RGB/ARGB
support. It is not limited to the developer's attached Corsair devices. Prefer
existing OS interfaces and avoid mandatory external drivers. LHM/PawnIO is an
optional backend, not a prerequisite for freeRGB. See `MAINBOARD_SUPPORT.md`.
The broader feature matrix is a reference backlog, not a completion claim. The
comparison baseline is FanControl V284; newly discovered functions extend this
list. Equal UI labels do not establish equal behavior or hardware support.

## Reference and investigation boundary

* [Official release repository](https://github.com/Rem0o/FanControl.Releases), V284,
  published 2026-10-18; [official documentation](https://getfancontrol.com/docs/).
* Official portable .NET 4.8 archive downloaded, not installed or executed:
  `FanControl_284_net_4_8.zip`, 12,191,400 bytes, SHA256
  `c631dd2605ca8d52027af74cc5b6df0079b171c56531b96907aa8fcf9dac1784`.
* REA 6.7.0 archive inventory Evidence:
  `ev_73a6e35deeea9081299684a4a3948a35ac85009be32d3af2f508c45e746d9128`.
  82 nodes/occurrences, including FanControl.exe, service, plugin and IPC libraries,
  LibreHardwareMonitorLib.dll. This establishes package contents, not runtime
  behavior, source code or drivers installed on this computer.
* Complete Evidence retained in ignored `target/fancontrol-evidence/`; REA's
  central-directory inventory has explicit coverage limits. CLI inventory worked;
  MCP registration drift and missing native engines did not require configuration
  changes for this task. No setup/driver/service changes were performed.
* Reference is proprietary with a restrictive license. Feature definitions use
  official public documentation. No FanControl code/resources/binaries are included
  in freeRGB, and no managed methods were decompiled in this investigation.

## Acceptance matrix

Feature headings below summarize the official docs; implementation and acceptance
columns describe freeRGB independently. `Preview` is never a hardware-control pass.

| Area | Target | Current freeRGB | Acceptance still needed |
|---|---|---|---|
| Setup | Guided discovery | Inventory | Guided channel identification |
| Pairing | Manual/automatic tachometer pairing | Candidate LINK map | Physical correlation |
| Calibration | Duty/RPM characterization | Implemented (simulated): channel test, 100/80/60/40 calibration and fine sweeps (2..20 % steps) stored per channel identity; chip-family simulator sweeps | Owner-board fine sweep (prompt 07) |
| Calibration | Startup/stall thresholds | Implemented (simulated): unrestricted-only stall sweep records stall and start duty per channel identity; never on pumps or below 40 % otherwise | Per-channel verified limits on owner board (prompt 07) |
| Calibration | Avoid resonance ranges | Implemented (simulated): `shaping.avoid` duty or RPM ranges (RPM via measured map), deterministic nearest-boundary skip, floor-aware | Validated ranges on real fans (prompt 07) |
| Control | Manual percentage | Implemented (simulated): `mode:manual` supervised flat curve, tier floors, critical override; shaping applies | Supervised hardware writes (prompt 07) |
| Control | Curve/channel assignment | Implemented (simulated): live assignments (curve, manual, linear, composite, RPM incl. shaping) remembered per hardware identity (chip, control, saved binding) and resumed only when unchanged | Owner-board resume acceptance (prompt 07) |
| Control | RPM targets | Implemented (simulated): open-loop map plus `closed_loop` PI correction with bounded integral/anti-windup and open-loop fallback on bad RPM telemetry; simulators per chip family | Closed-loop hardware acceptance (prompt 07) |
| Control | Duty floor/offset | Implemented (simulated): `shaping` offset and per-device min/max on any curve, then tier floor | Hardware acceptance |
| Control | Startup/stop behavior | Implemented (simulated): spin-up boost leaving 0 % or the stored stall duty; fan stop threshold only on unrestricted fans, never pumps | Pump-safe policy verified on owner board (prompt 07) |
| Control | Rise/fall limits | Implemented (simulated): curve `max_rise/fall_percent_per_s` timed controller; fail-safe bypasses limits | Hardware acceptance |
| Control | Hysteresis/delay | Implemented (simulated): temperature hysteresis on curves plus `shaping` response delay and up/down duty hysteresis with defined inclusive boundaries | Hardware acceptance |
| Curves | Linear | Implemented (simulated): `mode:linear` natively bound as a two-point curve; composite `linear` node | Hardware acceptance |
| Curves | Graph | Implemented (simulated): 2..64 points, linear or step interpolation, natively bound via `mode:curve` | Hardware acceptance |
| Curves | Constant | Evaluator, daemon storage + live preview API, Tauri editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Combination | Mix (max/min/avg/sum), fail-safe propagation, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Two-threshold switching | Trigger with hold time, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Following another output | Follow, cycle-checked, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Adaptive equilibrium | Implemented (simulated): composite `equilibrium` trend controller (target, gain, max change rate, look-ahead), cycle-checked, fail-safe restart at max | Hardware acceptance; boot-time service |
| Sensors | Combinations | Daemon evaluation/editor/storage | Hardware source acceptance |
| Sensors | Correction | Daemon evaluation/editor/storage | Native binding |
| Sensors | Rolling average | Daemon evaluation/editor/storage | Live acceptance |
| Sensors | Text-file import | Missing | Bounded reader/freshness |
| Hardware | Board coverage per fan-controller chip | Generated [FAN_COVERAGE.md](FAN_COVERAGE.md) (LHM v0.9.6 + hwmon drivers, CI drift check); 56 controllable chips simulated per family (channel test, plausibility, fault return, unlock); NCT6701D hardware-verified for the CHA_FAN2 scope only; community report template/importer | Owner-board acceptance of channel test/plausibility/unlock; community reports for other chips |
| Providers | CPU/GPU/board/storage | LINK telemetry; optional read-only Windows LHM provider | Driver-backed/multi-board acceptance; CPU/GPU/storage providers |
| GPU | Vendor control/release | Missing | NVIDIA/AMD capability handling |
| Plugins | External sensors/controls | RGB process plugins only | Cooling plugin contract/host |
| Profiles | Cooling configurations | RGB profiles only | Import/export and switching |
| Desktop | Tray/layout/themes | Partial GUI | Usability parity |
| Startup | Login/service modes | Login launcher | Approved headless service |
| CLI | Configuration/window/lifecycle | Basic API CLI | Cooling configuration commands |
| Distribution | Update/signing | Installer/update foundation | Signed live update acceptance |

Official hardware coverage depends on provider libraries and third-party plugins.
The target therefore includes extensible cooling providers, not a claim that all
motherboards/GPUs can be driven by the existing HID adapters. Motherboard access
may require a separately approved driver; AMD tuning side effects need explicit
handling before enabling control. Calibration must never stop a pump or treat an
unknown channel as a case fan.

## Implementation order

1. Build general motherboard provider/channel discovery with optional prerequisites;
   distinguish header capabilities from actual attached fan models, and fan power
   from lighting. Hardware writes require a separate independent supervisor and
   verified fallback. LINK work alone does not satisfy the general requirement.
2. Validate daemon sensor graphs with platform sources; add native providers and
   bounded file sources. No extra driver is installed by freeRGB today.
3. Add the remaining curve/controller types, calibration and RPM operation.
4. Add cooling profiles, plugins, guided setup, tray/service integration and
   cross-device/hardware acceptance. Existing user stage order remains applicable;
   calibration/control safety is required before feature activation.

Full equivalence remains unverified. A feature is complete only with its UI/API,
storage, missing-data behavior, tests and appropriate hardware acceptance.

## Status update 2026-10-24

Motherboard fan control (up to 16 concurrent live channels, stop all, resume after restart with identity check) and motherboard RGB (ASUS Aura USB all motherboard PIDs, Gigabyte RGB Fusion 2 USB, GUI header configuration, on by default) are no longer experimental; safety mechanisms are unchanged. ASRock Polychrome USB is driven too; MSI Mystic Light is identified only (separate MSI work); SMBus-only boards are not supported. Hardware validation: mock-tested only except CHA_FAN2 single-channel; see `MAINBOARD_RGB.md` and `COOLING_RUNTIME.md`.

## Status update 2026-11-04 (prompt 02, controller features)

Calibration, Control and Curves rows are implemented as pure policies in
`freergb-control` (`shaping`, `rpm::RpmPi`, `curves` linear/equilibrium nodes,
`FanCurve::interpolation`, `Calibration::sweep`) with daemon integration
(`ControlMode::{Shaped,Rpm,Sweep}`) and the chip-family simulators of
`crates/freergb-control/tests/chip_families.rs`. All of it is **simulated only**;
no row is hardware-accepted. API: [LOCAL_API.md](LOCAL_API.md#controller-curve-and-calibration-parameters-additive-2026-11-04).

Owner hardware steps for prompt 07 (ASUS PRIME X870-P WIFI, NCT6701D, owner
present, CPU below 60 °C, stop-all button ready):

1. Fine sweep: CHA_FAN2, Calibrate with "every 5 %". Expect 13 steps 100→40 %,
   restore to 100 %, then BIOS control; stored `tested` in `cooling-fan-support.json`.
2. Stall sweep: unlock CHA_FAN2 (unrestricted), "every 5 %" + "Find stall and start
   speed". Watch the fan stop and restart; compare `stall_percent`/`start_percent`
   with a stopwatch/visual check; confirm the sweep refuses on a pump-bound header.
3. Closed-loop RPM: target ~60 % of the measured range, `closed_loop` on; after
   30 s RPM within ±3 %; unplug nothing — instead pause the LHM provider to make
   RPM stale and confirm open-loop duty, then fault return on sensor loss.
4. Shaping on a curve: offset +10 %, max 80 %, delay 5 s, hysteresis 3/5 %, one
   avoided range around an audible resonance; verify with the live duty readout
   that duties never sit inside the range and changes wait for the delay.
5. Spin-up and fan stop (unrestricted only): stop below 20 %, spin-up 70 % / 1 s;
   confirm the fan restarts reliably from standstill; confirm fan stop is refused
   on a non-unlocked channel and on a pump.
6. Linear and step-graph curves via `mode:linear` / a step curve; equilibrium node
   holding 65 °C under a CPU load generator in a composite session; then a
   temperature source loss must give 100 % and the BIOS return.
7. Restart the daemon with a remembered linear/shaped assignment and confirm it
   resumes only with an unchanged identity.
