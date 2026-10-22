<p align="center">
  <img src="assets/brand/github-banner.svg" alt="freeRGB — Open-source RGB control for Windows and Linux" width="100%">
</p>

# freeRGB

**Open-source RGB control for Windows and Linux, powered by Rust.**

<!-- progress:start -->
**Delivery checklist: 51.4% complete — 19 of 37 milestones; 18 remaining.**

[View completed and remaining work](docs/STATUS.md). Automatically calculated from the delivery checklist; milestones are equally weighted, not an estimate of effort or release readiness.
<!-- progress:end -->

[![CI](https://img.shields.io/github/actions/workflow/status/leotrax3d/FreeRGB/ci.yml?style=flat-square&label=CI)](https://github.com/leotrax3d/FreeRGB/actions/workflows/ci.yml)
![Status](https://img.shields.io/badge/status-early%20development-orange?style=flat-square)
![Rust](https://img.shields.io/badge/rust-1.80%2B-blue?style=flat-square)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux-lightgrey?style=flat-square)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green?style=flat-square)
[![Downloads](https://img.shields.io/github/downloads/leotrax3d/FreeRGB/total?style=flat-square&color=10A37F&label=Downloads)](https://github.com/leotrax3d/FreeRGB/releases)
[![Views](https://hits.sh/github.com/leotrax3d/FreeRGB.svg?label=Views&color=10A37F&style=flat-square)](https://github.com/leotrax3d/FreeRGB)

freeRGB combines a background RGB service, a desktop interface and a local API.
The service keeps running when the interface closes. No cloud account or vendor
application is required for supported devices.

[Get started](#getting-started) · [Device support](#device-support) · [Project status](#project-status) · [Documentation](#documentation)

> [!IMPORTANT]
> Early development. Device support is limited, experimental integrations require
> explicit opt-in, and experimental motherboard fan control requires physical fan confirmation and UAC.
> Close other RGB controllers before accessing real devices.

## What freeRGB offers

- **Lighting:** 11 built-in effects, audio-reactive lighting, screen ambience,
  per-device brightness and placement on a shared effect canvas.
- **Custom effects:** a layered editor with blending, opacity, regions and a saved library.
- **Profiles:** saved configurations and optional Windows foreground-application rules.
- **Automation:** visual/JSON macro editing, run/cancel, delays, repeats and lighting
  actions; optional Windows keyboard/media output and global shortcuts.
- **Monitoring:** cached RPM/temperature readings, histories, sensor alerts,
  custom Celsius source graphs and redacted diagnostic exports.
- **Extensibility:** declarative device descriptions and explicitly trusted,
  isolated device-plugin processes.
- **Local control:** JSON-RPC over a Windows named pipe or Linux user socket.

Implementation and hardware acceptance are tracked separately below. Windows
capture/input features do not yet have Linux parity.

## Project status

The [delivery checklist](docs/STATUS.md) consolidates the product plan, worklist,
cooling execution plan and feature-specific documentation. Completed items describe
the named implementation milestone; acceptance and release milestones have their
own open tasks.

| Area | Available or implemented | Remaining |
|---|---|---|
| RGB engine and effects | Shared canvas, device workers, reconnect/hot-plug, effects and layered editor | Broader device coverage and extended combined-device soak tests |
| Desktop interface | Classic egui interface for devices, effects, profiles, macros and monitoring | Modern Tauri/React interface integration, release and feature parity |
| Profiles and automation | Persistence, Windows application rules, macro execution and opt-in shortcuts | Macro recording, device-specific macro keys and native interaction acceptance |
| Reactive keyboard lighting | Daemon overlay, explicit K70 CORE mapping, optional Windows capture | Real typing-to-LED acceptance, more layouts/keyboards and Linux capture |
| Monitoring and sensors | Histories, alerts, custom source graphs and optional protected motherboard reader | Verified source/channel identities and broader CPU/GPU/coolant coverage |
| Cooling | Curve editor/storage, daemon-owned protected sessions, explicit Classic live dialog and verified return | Public-path interactive acceptance, sustained hardware/fault validation, multi-channel support, modern-GUI live parity and release maintenance acceptance |
| Motherboard RGB/ARGB | Separate experimental ASUS USB backend | Real header colour/fallback acceptance, setup UI and more manufacturers |
| Plugins | Device descriptions, process lifecycle/API and virtual reference | Hardware plugin acceptance, effect plugins and installation/hot reload |
| Windows distribution | Installer, configuration preservation, CI lifecycle harness and verified update staging | Published signed release, cross-version update/rollback and interactive installer acceptance |
| Linux | Shared Rust core/IPC, CI, udev rules and systemd user unit | Hardware acceptance, capture/input parity and distribution packages |

**Next priorities:** finish protected cooling integration and fault acceptance;
expose accepted controls through the daemon and both interfaces; complete modern
GUI integration; validate distribution and expand hardware/platform coverage.

Existing hardware results include an 80% curve, 100/80/60/40% calibration and
verified return on one confirmed channel. The minimum recommendation remains
unverified. Same-version active service replacement passed; release upgrades,
rollback and uninstall remain open. The additive session API and Classic dialog
still need interactive public-path acceptance. See the
[current acceptance record](docs/COOLING_ACCEPTANCE.md),
[cooling execution plan](docs/COOLING_IMPLEMENTATION_PLAN.md) and
[protected bridge](docs/MAINBOARD_CONTROL_BRIDGE.md) for acceptance boundaries.

## Device support

Support is specific to a device/protocol, not an entire vendor.

| Device or integration | Status | Scope |
|---|---|---|
| Corsair K70 CORE RGB (`1b1c:1bfd`) | Validated | Per-key RGB; ISO/ANSI mapping |
| Corsair iCUE LINK System Hub (`1b1c:0c3f`) | Experimental, opt-in | Confirmed lighting on selected RX/LX fans, Titan RX AIO head and LCD ring; RPM watchdog |
| ASUS AURA USB (`0b05:19af`) | Experimental backend | Protocol/mock tests; physical RGB/ARGB header acceptance pending |
| Motherboard fan control | Experimental explicit sessions | Fan confirmation and UAC required; limited channel evidence, wider acceptance open |
| Other identified devices | Discovery only | Identification does not imply lighting, display or cooling control |

The AIO LCD display content and DDR5 RGB control are not supported.
For exact IDs, evidence and limitations, see [devices](docs/DEVICES.md),
[protocols](docs/PROTOCOLS.md) and [motherboard support](docs/MAINBOARD_SUPPORT.md).

## Getting started

### Build from source

Requires Rust 1.80+ and platform build dependencies. Windows needs the MSVC
toolchain and Visual Studio C++ Build Tools; Linux needs HID, GUI and audio/capture
development packages. See [installation](docs/INSTALL.md) for the complete list.

```bash
git clone https://github.com/leotrax3d/FreeRGB.git
cd FreeRGB
cargo build --release
```

### Try without hardware

Run the daemon and GUI in separate terminals:

```bash
cargo run --release -p freergb-daemon --bin freergbd -- --no-hardware --no-state --virtual-device
cargo run --release -p freergb-gui
```

### Connect supported hardware

Close competing RGB applications, then discover devices read-only:

```bash
cargo run --release -p freergb-cli -- discover
cargo run --release -p freergb-daemon --bin freergbd
```

Open the GUI from another terminal:

```bash
cargo run --release -p freergb-gui
```

The experimental iCUE LINK hub requires `--enable-link-hub` on the daemon.
Closing the GUI leaves the daemon running. Ctrl+C or `daemon.shutdown` requests
adapter-specific return to hardware control; failures must remain visible.

Windows installer builds and Linux service/permission setup are documented in
[INSTALL.md](docs/INSTALL.md). Update staging exists; a published release and a
successful real version-to-version update remain delivery milestones.

## Architecture

The daemon owns device access, effect rendering, state, profiles and control
policy. Desktop interfaces and the CLI use the same local API. Each device has
an isolated worker so a slow device can skip frames without blocking others.

The Classic interface uses egui. A separate Tauri 2/React interface is under
development; this README does not claim it has reached release parity.
Optional Windows motherboard access uses a protected service/native boundary.

See [architecture](docs/ARCHITECTURE.md), [decisions](docs/DECISIONS.md) and
[local API](docs/LOCAL_API.md).

## Safety and privacy

- Unsupported devices have no write adapter; experimental adapters require their
  documented activation. Detection alone never authorizes control.
- No firmware or flash writes. Cooling and lighting are separate control paths.
- LINK access is opt-in and uses an RPM watchdog. Productive motherboard cooling
  remains gated on independent supervision, verified return and hardware acceptance.
- Device return is adapter-specific. Experimental ASUS lighting fallback does
  not restore previous user settings. Unresolved cooling recovery blocks control
  and maintenance.
- Common competing RGB processes are detected; simultaneous access can disrupt
  vendor interfaces.
- The control API is local: named pipe on Windows, user socket on Linux.
  Process plugins are trusted executable code, **not sandboxed**.
- Ordinary RGB installation requires no motherboard driver. Optional Windows
  motherboard prerequisites require explicit setup opt-in.

See [security](docs/SECURITY.md), [plugin trust](docs/PLUGIN_SYSTEM.md) and
[cooling safety](docs/COOLING_SAFETY.md).

## Documentation

| Topic | Reference |
|---|---|
| Current delivery checklist and progress calculation | [Status](docs/STATUS.md) |
| Product scope and implementation history | [Product plan](docs/PRODUCT_PLAN.md) · [Worklist](docs/WORKLIST.md) |
| Setup and updates | [Installation](docs/INSTALL.md) · [Updates](docs/UPDATES.md) |
| Device evidence and contribution process | [Devices](docs/DEVICES.md) · [Protocols](docs/PROTOCOLS.md) · [Reverse engineering](docs/REVERSE_ENGINEERING.md) |
| Cooling implementation and remaining parity | [Execution plan](docs/COOLING_IMPLEMENTATION_PLAN.md) · [FanControl target](docs/FANCONTROL_PARITY.md) |
| Automation and interaction | [Macros](docs/MACROS.md) · [Application profiles](docs/APPLICATION_PROFILES.md) · [Keyboard reactions](docs/KEYBOARD_REACTIVE.md) |
| Sensors and troubleshooting | [Monitoring](docs/MONITORING.md) · [Custom sources](docs/CUSTOM_SENSORS.md) · [Diagnostics](docs/DIAGNOSTICS.md) |
| Development and verification | [API](docs/LOCAL_API.md) · [Plugins](docs/PLUGIN_SYSTEM.md) · [Testing](docs/TESTING.md) · [Handoff](docs/HANDOFF.md) |

## Contributing

Device reports, reproducible bugs and Linux testing are welcome. Include the
relevant version, platform, device IDs and redacted diagnostics. New adapters
need documented protocol evidence, simulator tests and explicit hardware acceptance.

Before submitting code:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For documentation changes, also run `python scripts/update-readme-progress.py --check`.
When changing delivery status, update [STATUS.md](docs/STATUS.md) with supporting
evidence and run `python scripts/update-readme-progress.py`.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Contributions use the same terms unless explicitly stated otherwise.

freeRGB is independent and is not affiliated with or endorsed by hardware vendors
or other RGB software projects. Product names belong to their respective owners.

