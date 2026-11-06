# Handoff — current entry point (2026-11-05)

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

## Product-completion run (2026-11-05, WS1–WS7)

A coordinated agent team worked through
[docs/promts/herdr-product-completion.md](promts/herdr-product-completion.md).
All seven PRs are merged into `main` in the prescribed order. Each was verified
locally before merge (fmt, workspace clippy `-D warnings`, workspace tests,
desktop typecheck/tests where touched). CI runs on GitHub in parallel and was
not used as the gate. A post-merge integration run on `main` was also green
locally. No UAC, installer runs, service changes, hardware writes or passwords
were used. Every result below is **simulated or fixture-tested only**, and no
STATUS box was checked.

| WS | PR | What landed | Simulated only / limits | Owner acceptance |
|---|---|---|---|---|
| WS4 updates & release tooling | [#21](https://github.com/leotrax3d/FreeRGB/pull/21) | Fault-injected download/verify/stage tests (interrupt, bad hash, disk full); cross-version swap/failed-start/rollback model; `freergb-module-tool` release command (build, pack, sign with external key, catalog, release notes, `--dry-run`) | Swap/rollback is a disk-backed model, nothing installed; no publishing | [UPDATES.md](UPDATES.md), [HARDWARE_MODULES.md](HARDWARE_MODULES.md) "Owner acceptance steps for local releases" |
| WS2 effect plugins | [#25](https://github.com/leotrax3d/FreeRGB/pull/25) | Versioned child-process effect host (`--effect-worker`), domain-separated signed effect catalog (existing owner public key), enable/disable, hot reload at frame boundaries, crash/trap fallback to previous effect, active uninstall | Fixture WASM effects and virtual canvases only; catalog signature with the real key not produced | [EFFECT_PLUGINS.md](EFFECT_PLUGINS.md) |
| WS6 motherboard RGB | [#24](https://github.com/leotrax3d/FreeRGB/pull/24) | ASUS per-header colour order and release fallback, header/LED detection report, config in both GUIs, import/export with profiles, byte-exact protocol tests | No packets sent to a real board | [MAINBOARD_RGB.md](MAINBOARD_RGB.md) |
| WS1 macros | [#26](https://github.com/leotrax3d/FreeRGB/pull/26) | Recorder state machine (`macros.record.*`), key down/up + text with coalescing/caps, privacy stops, draft-until-save, Tauri + Classic record UI, tray indicator, device-key assignments with validation | The Windows keyboard hook gives no physical device identity (`device_id` stays null). Per-device triggers are tested with synthetic events only | [MACROS.md](MACROS.md) |
| WS3 Linux parity | [#29](https://github.com/leotrax3d/FreeRGB/pull/29) | PulseAudio/PipeWire monitor audio, portal/X11 screen capture helper, read-only evdev input (same `InputEvent`), `.deb`/`.rpm`/AppImage scripts, udev rule, systemd user service docs | Built on Windows. Audio and evdev were cross-checked for `x86_64-unknown-linux-gnu`; screen capture (PipeWire/GLib) compiles only in Linux CI. The CI step snippet is in the PR description for the CI owner | [INSTALL.md](INSTALL.md) Linux "Owner acceptance steps" |
| WS5 desktop i18n/a11y | [#22](https://github.com/leotrax3d/FreeRGB/pull/22) | Dependency-free i18n (English + German, settings switch, parity test), keyboard navigation, screen-reader labels, shared empty/error states, foreground-app profile card | Cooling, macros and mainboard RGB views are not translated yet (add through the same framework, see [I18N.md](I18N.md)); screenshots were not regenerated | Check the German UI and screen-reader output in the real app; regenerate `docs/screenshots/` |
| WS7 user docs & website | [#20](https://github.com/leotrax3d/FreeRGB/pull/20) | [USER_GUIDE.md](USER_GUIDE.md) (install, first run, lighting, profiles, macros, plugins, fan safety, troubleshooting), website guide and coverage pages that read the coverage JSON | Coverage JSON files are only read | Proofread the published website |

Follow-ups found during the run:
* `cargo test -p freergb-daemon` on its own fails `tests/effect_marketplace.rs`,
  because that test needs the effect-worker binary built by the workspace. The
  full `cargo test --workspace` passes. Make the test self-sufficient.
* The marketplace and plugin process tests have tight timing and failed under
  heavy local load (they pass when rerun or isolated).
* Local builds: the dev box's `C:` SSD saturates with several parallel cargo
  builds. Per-worktree `target-dir`s on the NVMe `E:` drive (via a gitignored
  `.cargo/config.toml`) fixed this.

## What exists (simulated/unit-tested unless stated)

| Area | State | Details |
|---|---|---|
| RGB core, effects, profiles, audio/screen | Implemented | [ARCHITECTURE.md](ARCHITECTURE.md), [LOCAL_API.md](LOCAL_API.md) |
| Native devices | K70 CORE and iCUE LINK hardware-validated; ASUS Aura USB, Gigabyte Fusion 2, ASRock Polychrome mock-tested | [DEVICES.md](DEVICES.md), [MAINBOARD_RGB.md](MAINBOARD_RGB.md) |
| OpenRGB GPL process modules | 42 HID families in `modules/`, none hardware-accepted | [OPENRGB_PORTING.md](OPENRGB_PORTING.md), [OPENRGB_COMPATIBILITY.md](OPENRGB_COMPATIBILITY.md) |
| Hardware module system | Signed catalog, atomic install, brokered processes; SMBus bus-module SDK with signed child-process tests | [HARDWARE_MODULES.md](HARDWARE_MODULES.md), [SMBUS_SIMULATION.md](SMBUS_SIMULATION.md) |
| Cooling plugins | API 3 contract, policy-enforcing host, virtual + read-only iCUE LINK reference plugins (simulated) | [COOLING_PLUGINS.md](COOLING_PLUGINS.md) |
| Fan control | LHM tiers, multi-channel live sessions, composite curves, open-loop RPM targets, Linux hwmon, complete cooling profiles + import/export, headless `--cooling-service`, `freergb cooling` CLI, Tauri guided fan setup, tray stop-all, Classic composite editor | [COOLING.md](COOLING.md), [COOLING_SERVICE.md](COOLING_SERVICE.md), [FANCONTROL_PARITY.md](FANCONTROL_PARITY.md), [COOLING_ACCEPTANCE.md](COOLING_ACCEPTANCE.md), per-chip [FAN_COVERAGE.md](FAN_COVERAGE.md) |
| Macros, effect plugins | Macro recording + device keys; signed effect plugins with hot reload (simulated) | [MACROS.md](MACROS.md), [EFFECT_PLUGINS.md](EFFECT_PLUGINS.md) |
| GUIs | Tauri/React desktop (default, English/German) and Classic Win32/egui fallback | [GUI_MODERNIZATION.md](GUI_MODERNIZATION.md), [I18N.md](I18N.md), [INTEGRATION_CHECKLIST.md](INTEGRATION_CHECKLIST.md) |
| Distribution | Windows installer, updater with recovery simulation, module release tool, Linux `.deb`/`.rpm`/AppImage scripts | [INSTALL.md](INSTALL.md), [UPDATES.md](UPDATES.md), [USER_GUIDE.md](USER_GUIDE.md) |

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

Calibration owner step R5 (2026-11-06) needs a re-test after the bounded RPM
settling fix; slow spin-up and flat-top behavior are covered in unit tests and
the NCT6701D simulator. See the [acceptance record](COOLING_ACCEPTANCE.md#owner-session-2026-11-06-r5-calibration-settling).
Loaded linear-session acceptance R5b also needs owner re-test after aligning
native telemetry freshness; see [timing and safety bounds](COOLING_RUNTIME.md).

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, plus the feature-gated process tests listed in
`.github/workflows/ci.yml`. Details and history: [TESTING.md](TESTING.md).

## Licensing rule

Core is MIT/Apache-2.0. Code ported from OpenRGB (GPL-2.0-or-later) lives only in
separately built process modules under `modules/`. FanControl is used only as a
feature reference (no code). See [DECISIONS.md](DECISIONS.md).
