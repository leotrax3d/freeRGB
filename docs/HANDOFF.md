# HANDOFF – current state (2026-10-18)

## Phase
Roadmap phases 1–8 done: first real RGB device (K70 CORE) validated, background
service with local API working end-to-end. Phase 9 started: native GUI (egui) talking only to the API. Next: plugin system.

## Done
* Tool/environment inventory (no "Reverse Engineer Anything" MCP server available).
* SignalRGB 2.5.80 inventory (`REVERSE_ENGINEERING.md`), hardware inventory (`DEVICES.md`).
* Rust 1.99.1 + MSVC Build Tools installed (approved by the user).
* Corsair Bragi protocol implemented and validated on the K70 CORE (`PROTOCOLS.md`).
* Project renamed to freeRGB; private repo `github.com/leotrax3d/FreeRGB`.
* iCUE LINK hub: read-only probe, sensors, lighting of 3 RX fans + Titan AIO + LCD ring, opt-in
  daemon device with RPM watchdog (15-min soak passed).
* `freergbd`: one worker thread per device behind a global HID gate (hub 52 FPS, K70 19 FPS), reconnect with back-off, hot-plug rescan, JSON-RPC over a
  named pipe / Unix socket, clean release to hardware mode on exit; effect/brightness persisted across restarts.

## Working components [confirmed]
| Crate | Content |
|---|---|
| `freergb-core` | Rgb/Hsv, colour correction, LedLayout, Canvas, Frame |
| `freergb-device-api` | `HidTransport`, `RgbDevice`, `VirtualDevice`, `MockTransport` |
| `freergb-effects` | 8 effects (Solid, RainbowWave, Breathing, Gradient, ColorCycle, RadialRainbow, Scanner, Twinkle), serializable `EffectSpec` with defaults |
| `freergb-hid` | hidapi discovery/transport, report descriptor parser |
| `freergb-protocols` | Corsair Bragi adapter (K70 CORE), iCUE LINK hub (probe, lighting, `LinkHubDevice`, `RpmWatchdog`), adapter registry |
| `freergb-daemon` | engine, JSON-RPC API, IPC, hardware scan; binary `freergbd` |
| `freergb-cli` | binary `freergb`: `discover`, `demo-virtual`, `bragi …`, `link-probe`, `link-light-test`, `link-restore`, `ctl` |
| `freergb-client` | blocking JSON-RPC client (used by CLI and GUI) |
| `freergb-gui` | binary `freergb-gui` (egui): devices, all effects with parameters, brightness, profiles, stop daemon |

## Tests
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` → 69 tests passing locally (Windows); GitHub Actions CI runs fmt/clippy/test/release build on windows-latest and ubuntu-latest (green since 2026-10-18). Hardware on Linux not yet tested.
Hardware results: `DEVICES.md#validation-log`.

## Open issues / questions
* K70 CORE ~19–22 FPS with acknowledged streaming (device-side limit). Pipelining hangs the
  keyboard after up to 2.5 min even with serialised HID access → not usable.
* K70 dropped out after ~10 min in a 15-min soak: device closed its lighting handle (no keepalive
  ping). Keepalive + plugin pacing added (`70fe64b`); 20-min soak pending.
* While the K70 streams, the hub drops from ~52 to ~28 FPS (std Mutex is not fair, the K70
  worker re-acquires the HID gate quickly) – consider a fair gate.
* Each re-plug of the K70 was needed after a vendor-interface hang; a software reset path
  (e.g. re-open + mode reset) is not known yet.
* `1b1c:1bf0` (no USB interface) – origin unknown.
* Full `key-walk` over all 107 keys not yet visually verified.
* `freergbd` is not installed as a service/autostart (needs user approval: system change).
* The SignalRGB app is currently stopped (its service still runs). Restart with
  `%LOCALAPPDATA%\VortxEngine\SignalRgbLauncher.exe`.

## Next tasks
1. GUI: layout view, profiles, per-device brightness; test on Linux.
2. Declarative device descriptions (TOML) as first plugin stage.
3. iCUE LINK hub: Commander DUO ARGB ports (if any); longer soak (hours) before making `--enable-link-hub` a recommended default.
4. Test hardware access on a real Linux machine with `packaging/linux/60-freergb.rules` (see `INSTALL.md`).
5. Named profiles on top of the persisted state; autostart proposal.

## Entry points
`crates/freergb-daemon/src/main.rs`, `crates/freergb-protocols/src/corsair_bragi/device.rs`,
`crates/freergb-cli/src/main.rs`. SignalRGB plugins (analysis source):
`%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins`.
