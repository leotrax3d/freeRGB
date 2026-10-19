<div align="center">

# freeRGB

**Open, lightweight RGB control for Windows and Linux – written in Rust.**

An independent alternative to SignalRGB: a small background service drives your RGB
hardware directly, a native GUI and a local API control it. No cloud account, no telemetry,
no vendor software required.

[![CI](https://github.com/leotrax3d/FreeRGB/actions/workflows/ci.yml/badge.svg)](https://github.com/leotrax3d/FreeRGB/actions/workflows/ci.yml)
![Status](https://img.shields.io/badge/status-early%20development-orange)
![Rust](https://img.shields.io/badge/rust-1.80%2B-blue)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux-lightgrey)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)

</div>

> [!WARNING]
> freeRGB is **early-stage software**. It talks to real hardware, including fan and pump
> controllers. Read [Safety](#safety) before you run it, and never run it at the same time
> as SignalRGB or iCUE.

---

## Project status

*Last updated: 2026-10-19*

| Area | State |
|---|---|
| RGB core (colours, layouts, canvas, frames) | ✅ done, tested |
| Effects | ✅ 11 built-in (Solid, Rainbow Wave, Breathing, Gradient, Colour Cycle, Radial Rainbow, Scanner, Twinkle, Audio Pulse, Audio Spectrum, Screen Ambience) + custom layered effects |
| Effect editor | ✅ GUI layer editor (up to 8 layers, blend modes, opacity, regions), named library |
| Background service `freergbd` | ✅ one worker thread per device, reconnect, hot-plug, state + profiles persisted |
| Local API (JSON-RPC over named pipe / Unix socket) | ✅ devices, effects, colour, brightness, profiles, diagnostics |
| Native GUI (egui) | 🟡 devices with per-device brightness, drag-and-drop canvas layout, all effects with their parameters, profiles |
| Plugin system | 🟡 TOML device descriptions + opt-in process device plugins, start/stop GUI/API, Rust virtual reference – [docs/PLUGIN_SYSTEM.md](docs/PLUGIN_SYSTEM.md) |
| Audio reactivity | ✅ Windows (WASAPI loopback), confirmed live on K70 + fans; Linux untested · `freergb audio-meter` |
| Screen ambience | ✅ Windows (primary monitor, 64×36, ~30 FPS), confirmed live; Linux untested · `freergb screen-probe` |
| Linux | 🟡 builds and tests pass in CI; hardware access not yet tested |

**Workspace:** 12 crates · Windows/Ubuntu CI configured. Current work and verified
results: [docs/WORKLIST.md](docs/WORKLIST.md), [docs/TESTING.md](docs/TESTING.md).

Process plugin demo (after a release build):
`freergbd --no-hardware --no-state --virtual-device --plugin examples/plugins/virtual-windows.json`.
Use the Linux manifest on Linux. Native plugins are explicitly trusted code, not sandboxed.

### Supported hardware

Only devices whose protocol has been verified on real hardware are ever written to.
Everything else is detected and listed, but never touched.

| Device | Status | Notes |
|---|---|---|
| Corsair **K70 CORE RGB** (`1b1c:1bfd`) | ✅ **Validated** | Per-key lighting (107 keys, ISO/ANSI), ~21 FPS, 20-min soak passed |
| Corsair **iCUE LINK System Hub** (`1b1c:0c3f`) | 🧪 **Experimental** (opt-in) | Lighting confirmed for RX/LX fans, Titan RX AIO pump head and LCD ring; ~50 FPS; fan/pump RPM watchdog |
| Corsair iCUE LINK child devices | ✅ detected | RX, LX, QX fans, Titan AIOs, LCD; Commander DUO recognised (no LEDs) |
| Corsair iCUE LINK AIO LCD screen (`1b1c:0c4e`) | 🔍 identified | Display content not supported |
| Corsair Vengeance RGB DDR5 (SMBus) | 🔍 identified | Deferred: SMBus writes near SPD are risky |
| ASUS AURA USB controller (`0b05:19af`) | 🔍 identified | Next vendor candidate |
| Razer Cobra (`1532:00a3`) | 🔍 identified | |

Full details, test logs and confidence levels: [docs/DEVICES.md](docs/DEVICES.md) and
[docs/PROTOCOLS.md](docs/PROTOCOLS.md).

---

## Quick start

Requirements: Rust 1.80+ (stable). On Windows the MSVC toolchain; on Linux `libudev-dev`
and the usual X11/Wayland development packages for the GUI.

```bash
git clone https://github.com/leotrax3d/FreeRGB.git
cd FreeRGB
cargo build --release
```

Quit SignalRGB / iCUE first. Then:

```bash
# What is connected, and what would freeRGB drive? (read-only)
cargo run --release -p freergb-cli -- discover

# Start the background service (add --enable-link-hub for iCUE LINK fans/AIO)
cargo run --release -p freergb-daemon --bin freergbd

# Control it from a second terminal …
cargo run --release -p freergb-cli -- ctl color.set '{"color":"#00ffff"}'
cargo run --release -p freergb-cli -- ctl profiles.save '{"name":"Evening"}'

# … or with the GUI
cargo run --release -p freergb-gui
```

Stopping the service (`ctl daemon.shutdown`, Ctrl+C, or the GUI button) hands every device
back to its own hardware lighting.

Installing the binaries, Linux udev rules and a systemd user service: [docs/INSTALL.md](docs/INSTALL.md).

### Command-line tools

| Command | What it does |
|---|---|
| `freergb discover` | Lists HID devices and the adapter that would drive them (read-only) |
| `freergb bragi probe` | Reads firmware, mode and layout of a Corsair Bragi keyboard (read-only) |
| `freergb bragi color #ff0000` / `effect rainbow` / `key-walk` | Short hardware tests on the K70 CORE |
| `freergb link-probe [--software-window]` | iCUE LINK hub: firmware, child devices, fan RPM, temperatures |
| `freergb link-light-test` | Reversible colour test on the iCUE LINK chain with RPM watchdog |
| `freergb link-restore` | Puts the iCUE LINK hub back into hardware mode |
| `freergb validate-device <file.toml>` | Checks a device description without touching hardware |
| `freergb ctl <method> [json]` | Calls any [local API](docs/LOCAL_API.md) method on the running service |

---

## How it works

```
freergb-gui ─┐                                   ┌─ worker ─ K70 CORE
freergb-cli ─┼─ JSON-RPC (named pipe / socket) ─ freergbd ─ engine ─┼─ worker ─ iCUE LINK hub
other apps ──┘                                   └─ worker ─ …
```

* **Engine:** an effect renders onto a shared canvas; every device samples its region,
  gets brightness/colour correction applied, and receives the newest frame on its own
  worker thread. A slow device skips frames instead of slowing the others down.
* **Protocols** are pure, unit-tested packet encoders plus a small state machine per device,
  developed against simulators before they ever touch hardware.
* **Local API** only — no network port. Windows named pipe `\\.\pipe\freergb`, Linux socket
  `$XDG_RUNTIME_DIR/freergb.sock` (mode 0600).

| Crate | Purpose |
|---|---|
| `freergb-core` | Colours (RGB/HSV), colour correction, LED layouts, canvas, frames |
| `freergb-device-api` | Device and transport traits, virtual device, mock transport |
| `freergb-protocols` | Vendor adapters (Corsair Bragi, iCUE LINK), adapter registry |
| `freergb-hid` | HID discovery and transport (hidapi, native backends) |
| `freergb-effects` | Built-in effects |
| `freergb-daemon` | `freergbd`: engine, API, IPC, hardware scan, state and profiles |
| `freergb-client` | Blocking client for the local API |
| `freergb-cli` | `freergb` command-line tool |
| `freergb-gui` | Native GUI (egui/eframe) |

More in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/DECISIONS.md](docs/DECISIONS.md).

---

## Safety

freeRGB is written to fail safe:

* **Unknown devices are never written to.** Detection is read-only; only adapters marked
  *Experimental* or *Validated* are opened.
* **No firmware or flash writes.** Device property writes go through an allowlist (today
  only the hardware/software mode switch).
* **Cooling hardware is opt-in.** The iCUE LINK hub is only driven with `--enable-link-hub`.
  While freeRGB controls it, an RPM watchdog checks fans and pump every 2 s and returns the
  hub to hardware mode on any slowdown, then refuses to reconnect until restart.
* **Devices are handed back** to hardware mode on every shutdown path and verified by read-back.
* **Other RGB software is detected.** While SignalRGB, iCUE or OpenRGB runs, `freergbd` hands
  all hardware back and leaves it alone (shown in the GUI); it resumes when they are closed.
  Concurrent access can lock up a device's vendor interface until it is re-plugged.

Details: [docs/SECURITY.md](docs/SECURITY.md).

---

## Roadmap

- [x] Platform-independent RGB core with tests
- [x] Virtual device and simulators
- [x] First real device (Corsair K70 CORE)
- [x] Background service with local API, state and profiles
- [x] iCUE LINK hub: fans, AIO, LCD ring (experimental)
- [x] Basic native GUI
- [ ] Longer hardware soak tests, then iCUE LINK out of experimental
- [x] Declarative device descriptions (first plugin stage)
- [x] Per-device placement on the effect canvas (layout editor in the GUI)
- [ ] More vendors: ASUS AURA, Razer, Logitech, …
- [x] Audio-reactive effects (pulse, spectrum)
- [x] Screen ambience
- [x] Effect editor (layers, blend modes, library)
- [x] Linux udev rules and systemd user unit ([packaging/linux](packaging/linux))
- [ ] Hardware testing on Linux, packaging

Full roadmap: [docs/ROADMAP.md](docs/ROADMAP.md).

---

## How devices are added

New device support follows a fixed, reproducible process
([docs/REVERSE_ENGINEERING.md](docs/REVERSE_ENGINEERING.md)):

1. Identify the device (VID/PID, HID usage pages) – read-only.
2. Document the protocol with sources and a confidence level per field.
3. Implement it as a pure packet encoder and test it against a simulator.
4. Verify read-only on hardware (the device must report its own IDs back correctly).
5. Run reversible colour tests and confirm visually, then mark the device *Validated*.

No code from other RGB software is copied; only interface facts are used, with their
source documented.

---

## Contributing

Bug reports and device information are welcome, especially:

* `freergb discover --all` output for devices that are not supported yet,
* results of the read-only probes on other Corsair devices,
* testing on Linux.

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Current internal state and next steps: [docs/HANDOFF.md](docs/HANDOFF.md).

---

## Disclaimer

freeRGB is an independent project. It is not affiliated with, endorsed by or supported by
Corsair, WhirlwindFX (SignalRGB), ASUS, Razer or any other hardware vendor. All product
names are trademarks of their respective owners. Use at your own risk.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in freeRGB by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
