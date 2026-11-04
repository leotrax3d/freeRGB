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
| Calibration | Duty/RPM characterization | Calibration and channel-test duty/RPM pairs stored per channel identity | Finer duty steps; hardware acceptance |
| Calibration | Startup/stall thresholds | Missing | Per-channel verified limits |
| Calibration | Avoid resonance ranges | Missing | Validated avoidance policy |
| Control | Manual percentage | Missing | Supervised hardware writes |
| Control | Curve/channel assignment | Preview | Stable identity bindings |
| Control | RPM targets | Open-loop `mode:rpm` from stored calibration/channel-test map, live panel input | Closed-loop correction; hardware acceptance |
| Control | Duty floor/offset | Basic curve floor | Per-device constraints |
| Control | Startup/stop behavior | Missing | Pump-safe policy |
| Control | Rise/fall limits | Missing | Timed controller |
| Control | Hysteresis/delay | Basic hysteresis | Delay/boundary behavior |
| Curves | Linear | Graph preview supports two points | Native binding |
| Curves | Graph | Preview/editor | Broader shape support |
| Curves | Constant | Evaluator, daemon storage + live preview API, Tauri editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Combination | Mix (max/min/avg/sum), fail-safe propagation, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Two-threshold switching | Trigger with hold time, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Following another output | Follow, cycle-checked, preview API, editor, live sessions (`mode:composite`) | Hardware acceptance; boot-time service |
| Curves | Adaptive equilibrium | Missing | Trend controller/validation |
| Sensors | Combinations | Daemon evaluation/editor/storage | Hardware source acceptance |
| Sensors | Correction | Daemon evaluation/editor/storage | Native binding |
| Sensors | Rolling average | Daemon evaluation/editor/storage | Live acceptance |
| Sensors | Text-file import | Bounded shell-free reader (4 KiB, bounds, scale, mtime timeout, no network paths by default), API + Tauri editor, missing → 100 % ([details](COOLING_SOURCES_GPU.md)) | Real-world file producer acceptance |
| Hardware | Board coverage per fan-controller chip | Generated [FAN_COVERAGE.md](FAN_COVERAGE.md) (LHM v0.9.6 + hwmon drivers, CI drift check); 56 controllable chips simulated per family (channel test, plausibility, fault return, unlock); NCT6701D hardware-verified for the CHA_FAN2 scope only; community report template/importer | Owner-board acceptance of channel test/plausibility/unlock; community reports for other chips |
| Providers | CPU/GPU/board/storage | LHM CPU/GPU/board/storage (Windows, optional), hwmon k10temp/coretemp/amdgpu/nvme/drivetemp (Linux), LINK liquid; categorized `cooling.sensors.catalog`, all selectable in curves | Owner-hardware source acceptance; multi-board |
| GPU | Vendor control/release | NVML backend (NVIDIA), amdgpu via hwmon (Linux); journal + release on stop/pause/crash/fault; simulated tests; Tauri panel | NVML hardware acceptance; elevated GPU worker; AMD ADLX (Windows) missing; Intel Arc read-only |
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
