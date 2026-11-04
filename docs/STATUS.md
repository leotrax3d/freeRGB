# Delivery status

This is the canonical, deduplicated checklist used for the README percentage.
It consolidates the current product documents as of 2026-10-22. Historical plans
and logs remain evidence, not additional tasks in the denominator.

Each checkbox represents one named milestone with equal weight. The percentage
is completed milestones / all listed milestones, rounded to one decimal place.
It is **not** an estimate of engineering effort, device coverage, test pass rate
or release readiness. Implementation and acceptance are separate milestones.
Partial work stays unchecked. Adding scope may lower the percentage.

When completing a milestone, update its evidence and checkbox together. Do not
infer completion from a build, simulator, detected device or old pending CI note.

## Lighting and desktop

- [x] RGB core, shared canvas, device workers and reconnect/hot-plug. Evidence: [architecture](ARCHITECTURE.md), [worklist](archive/WORKLIST.md).
- [x] Eleven effects, layered editor and saved effect library. Evidence: [product plan](PRODUCT_PLAN.md), [API](LOCAL_API.md).
- [x] Saved profiles and opt-in Windows application rules. Evidence: [application profiles](APPLICATION_PROFILES.md).
- [x] Windows audio-reactive and screen capture implementation. Evidence: [product plan](PRODUCT_PLAN.md), [testing](TESTING.md).
- [x] Classic desktop controls and local daemon API. Evidence: [architecture](ARCHITECTURE.md), [API](LOCAL_API.md).
- [x] Reactive keyboard overlay, K70 mapping and optional Windows helper. Evidence: [keyboard reactions](KEYBOARD_REACTIVE.md).
- [ ] Real keyboard typing, foreground-profile switching and GUI interaction acceptance. Evidence: [worklist](archive/WORKLIST.md), [testing](TESTING.md).
- [ ] Modern Tauri/React interface integration, release and feature parity. Integration merged into local `main` with user approval on 22 October 2026; visual/hardware/Linux and installer lifecycle acceptance remain open. Evidence: [integration checklist](INTEGRATION_CHECKLIST.md).

## Hardware and extensibility

- [x] K70 CORE per-key lighting hardware validation. Evidence: [devices](DEVICES.md).
- [x] Experimental iCUE LINK lighting implementation and selected-device acceptance. Evidence: [devices](DEVICES.md), [protocols](PROTOCOLS.md).
- [x] Declarative device descriptions and process-device plugin foundation. Evidence: [plugins](PLUGIN_SYSTEM.md).
- [ ] Extended simultaneous-device soak and LINK promotion beyond experimental. Evidence: [worklist](archive/WORKLIST.md).
- [ ] Hardware process-plugin acceptance, effect plugins and install/hot reload. Evidence: [plugins](PLUGIN_SYSTEM.md).
- [ ] ASUS header colour/fallback acceptance and configuration UI. Evidence: [mainboard RGB](MAINBOARD_RGB.md).
- [x] Downloadable hardware modules: signed catalog, verified atomic install, brokered module processes and setup UI (simulation and process tests; first module ports OpenRGB Razer). Evidence: [hardware modules](HARDWARE_MODULES.md).
- [ ] Razer Chroma module hardware acceptance and first published signed module release (needs release key). Evidence: [hardware modules](HARDWARE_MODULES.md).
- [ ] Additional vendor/device lighting adapters with hardware acceptance. Evidence: [product plan](PRODUCT_PLAN.md), [mainboard support](MAINBOARD_SUPPORT.md).

## Automation and monitoring

- [x] Macro editor, run/cancel, repeats and opt-in Windows output/shortcuts. Evidence: [macros](MACROS.md), [worklist](archive/WORKLIST.md).
- [x] Sensor histories, warning rules and redacted diagnostics. Evidence: [monitoring](MONITORING.md), [diagnostics](DIAGNOSTICS.md).
- [x] Daemon-owned custom Celsius source graphs. Evidence: [custom sensors](CUSTOM_SENSORS.md).
- [ ] Macro recording and device-specific macro key assignment. Evidence: [macros](MACROS.md), [product plan](PRODUCT_PLAN.md).
- [ ] Real native macro output/hotkey acceptance. Evidence: [testing](TESTING.md).
- [ ] Broader verified CPU/GPU/coolant sources and sensor warning acceptance. Evidence: [monitoring](MONITORING.md), [product plan](PRODUCT_PLAN.md).

## Motherboard cooling

- [x] Curve model/editor/storage and synthetic calibration policies. Evidence: [API](LOCAL_API.md), [cooling safety](COOLING_SAFETY.md).
- [x] Optional protected read-only provider and explicit channel annotations. Evidence: [broker](MAINBOARD_BROKER.md), [bindings](COOLING_CHANNEL_BINDINGS.md).
- [x] Protected native sessions, durable recovery and limited fixed-100% worker-crash evidence. Evidence: [recovery](MAINBOARD_CONTROL_RECOVERY.md), [runtime](COOLING_RUNTIME.md).
- [x] Rust execution host and authenticated commissioning bridge, including limited 80% curve/calibration evidence. Evidence: [bridge](MAINBOARD_CONTROL_BRIDGE.md), [acceptance record](COOLING_ACCEPTANCE.md); public-path interactive acceptance remains.
- [ ] Sustained variable-duty calibration/curve acceptance and verified operating limits. Limited CHA_FAN2 calibration ran; recommendation remains hardware_verified=false. Evidence: [acceptance record](COOLING_ACCEPTANCE.md).
- [ ] Independent production fault supervision and service/boot/driver-hang recovery acceptance. Evidence: [runtime](COOLING_RUNTIME.md).
- [ ] Production maintenance drain and active-control upgrade/uninstall/rollback acceptance. Handshake implemented; same-version active replacement passed, release/fault/uninstall acceptance remains. Evidence: [maintenance](COOLING_MAINTENANCE.md), [acceptance record](COOLING_ACCEPTANCE.md).
- [ ] Public daemon cooling methods and both GUI integrations after acceptance. Additive session API and Classic dialog implemented; public-path interactive acceptance and modern-GUI live-session parity remain. Evidence: [API](LOCAL_API.md), [acceptance record](COOLING_ACCEPTANCE.md).
- [ ] Multiple native channels and additional validated motherboard backends. Tiered LHM support (verified/LHM-supported/unrestricted), multi-channel journal, background control, guided channel test, unrestricted unlock, manual duty, step limits and cooling profiles are implemented on `main` with synthetic tests; nothing is hardware-confirmed yet. Per-chip coverage matrix with family simulators and community report import exists; only NCT6701D/CHA_FAN2 has hardware evidence. Evidence: [broad support plan](COOLING_BROAD_SUPPORT_PLAN.md), [fan coverage](FAN_COVERAGE.md).
- [ ] Remaining controller types, RPM operation, guided setup and cooling profiles/plugins. Controller, curve and calibration features (shaping, closed-loop RPM, stall sweep, linear/step/equilibrium) are implemented and simulated only; hardware acceptance is prompt 07. Evidence: [FanControl target](FANCONTROL_PARITY.md).

## Distribution and platforms

- [x] Windows installer, launcher, optional prerequisites and CI lifecycle harness. Evidence: [installation](INSTALL.md), [worklist](archive/WORKLIST.md); cross-version acceptance is separate.
- [x] Release update selection, checksum verification, staging and optional checks. Evidence: [updates](UPDATES.md).
- [x] Windows/Linux CI plus Linux udev and systemd packaging foundations. Evidence: [worklist](archive/WORKLIST.md), [installation](INSTALL.md).
- [ ] Interactive installer/vendor uninstall, prerequisite UAC/reboot and clean-machine acceptance. Evidence: [installation](INSTALL.md), [mainboard installation](MAINBOARD_INSTALLATION.md).
- [ ] Published signed release and real cross-version update/failure recovery. Evidence: [updates](UPDATES.md), [product plan](PRODUCT_PLAN.md).
- [ ] Real Linux hardware acceptance, audio/screen/input parity and distribution packages. Evidence: [product plan](PRODUCT_PLAN.md), [worklist](archive/WORKLIST.md).

## Reconciliation notes

The older roadmap/product-plan tables and append-only handoff/testing logs contain
superseded pending statements. Later worklist entries establish implemented global
shortcuts, installer lifecycle runs and Linux CI results. The cooling execution,
runtime, bridge and maintenance documents establish newer protected commissioning
work; additive public sessions and Classic controls now exist, while their
interactive acceptance and modern-GUI live-session parity remain open. Those later, specific records take
precedence over older general statements; unresolved acceptance stays open.

The README deliberately omits personal system inventories, individual fan RPM
measurements, local paths and CI run-by-run history. Device compatibility remains
documented separately.

## Automatic README text

Run `python scripts/update-readme-progress.py` after editing this checklist.
The workflow checks pull requests and updates the README after pushes to main
affecting documentation or the generator. It can also be run manually.
It changes only the marked README block and commits only when that block differs.

GitHub READMEs cannot run a live JavaScript widget or embed an application.
This is generated Markdown text, updated after the workflow completes and visible
when the page is reloaded; it does not refresh an already-open page in real time.
No image, badge, external counter or hosted service is used for progress.

Automation reads checkbox state; it cannot decide whether a feature is complete.
Contributors must keep this checklist consistent with implementation evidence.
If branch protection forbids the bot's direct commit, run the generator in the
same pull request; the pull-request check enforces that the text is current.


