# Handoff — current entry point (2026-11-02)

Start here. This page says where things are; [STATUS.md](STATUS.md) is the only
checklist for what is done. Older handoff, roadmap and worklist logs are in
[archive/](archive/) and are not current.

## Repository state

* All development lines are consolidated on `main` (PR #12, after
  release `0.1.0-beta.4`). There are no open feature branches.
* Work in progress that is **not** on `main`, kept on GitHub:
  * `module/mountain-keyboard`, `module/thingm-blink` — unfinished OpenRGB HID
    ports (uncommitted worktree state saved as-is, never built or tested).
  * `backup/modules-integration-remote`, `backup/modules-integration-wip` —
    the parallel module implementation that was deliberately not merged
    (duplicates the modules on `main`).
  * `backup/tauri-gui`, `backup/website-live-demo`,
    `claude/adoring-hopper-p5zi8e` — superseded; their content is on `main`
    in rewritten form. Keep only for reference.
* Rule for new work: one branch per task from `main`, one PR, delete the
  branch after merge. Do not start parallel implementations of the same area.

## What exists (simulated/unit-tested unless stated)

| Area | State | Details |
|---|---|---|
| RGB core, effects, profiles, audio/screen | Implemented | [ARCHITECTURE.md](ARCHITECTURE.md), [LOCAL_API.md](LOCAL_API.md) |
| Native devices | K70 CORE and iCUE LINK hardware-validated; ASUS Aura USB, Gigabyte Fusion 2, ASRock Polychrome mock-tested | [DEVICES.md](DEVICES.md), [MAINBOARD_RGB.md](MAINBOARD_RGB.md) |
| OpenRGB GPL process modules | 42 HID families in `modules/`, none hardware-accepted | [OPENRGB_PORTING.md](OPENRGB_PORTING.md), [OPENRGB_COMPATIBILITY.md](OPENRGB_COMPATIBILITY.md) |
| Hardware module system | Signed catalog, atomic install, brokered processes; SMBus bus-module SDK with signed child-process tests | [HARDWARE_MODULES.md](HARDWARE_MODULES.md), [SMBUS_SIMULATION.md](SMBUS_SIMULATION.md) |
| Cooling plugins | API 3 contract, policy-enforcing host, virtual + read-only iCUE LINK reference plugins (simulated) | [COOLING_PLUGINS.md](COOLING_PLUGINS.md) |
| Fan control | LHM tiers, multi-channel live sessions, composite curves, open-loop RPM targets, Linux hwmon, complete cooling profiles + import/export, headless `--cooling-service`, `freergb cooling` CLI | [COOLING.md](COOLING.md), [COOLING_SERVICE.md](COOLING_SERVICE.md), [FANCONTROL_PARITY.md](FANCONTROL_PARITY.md), [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md), per-chip [FAN_COVERAGE.md](FAN_COVERAGE.md) |
| GUIs | Tauri/React desktop (default) and Classic Win32/egui fallback | [GUI_MODERNIZATION.md](GUI_MODERNIZATION.md), [INTEGRATION_CHECKLIST.md](INTEGRATION_CHECKLIST.md) |
| Distribution | Windows installer, updater, Linux packaging foundations | [INSTALL.md](INSTALL.md), [UPDATES.md](UPDATES.md) |

## Biggest open gaps

1. **Hardware acceptance** — almost everything beyond K70/LINK is simulated.
   This is the main release blocker.
2. **OpenRGB coverage** — binding goal is 100 % of OpenRGB devices
   ([PARITY_EXPANSION_PLAN.md](PARITY_EXPANSION_PLAN.md)): ~79 HID families
   plus all non-HID transports (SMBus RAM, GPU I2C, serial, network, SDK
   protocol) remain.
3. **FanControl parity** — controller/curve/calibration features (incl. adaptive
   equilibrium) are simulated only; Classic composite-curve editor; text-file
   sensors and GPU fans (NVML) exist but are not hardware-accepted, AMD ADLX is
   missing ([COOLING_SOURCES_GPU.md](COOLING_SOURCES_GPU.md)) ([FANCONTROL_PARITY.md](FANCONTROL_PARITY.md)).
4. **Release** — signed module release, cross-version update and clean-machine
   installer acceptance ([STATUS.md](STATUS.md#distribution-and-platforms)).

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, plus the feature-gated process tests listed in
`.github/workflows/ci.yml`. Details and history: [TESTING.md](TESTING.md).

## Licensing rule

Core is MIT/Apache-2.0. Code ported from OpenRGB (GPL-2.0-or-later) lives only in
separately built process modules under `modules/`. FanControl is used only as a
feature reference (no code). See [DECISIONS.md](DECISIONS.md).
