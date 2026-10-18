# HANDOFF – current state (2026-10-18)

## Phase
Roadmap phases 1–8 done: first real RGB device (K70 CORE) validated, background
service with local API working end-to-end. Next: phase 9 (GUI, plugin system).

## Done
* Tool/environment inventory (no "Reverse Engineer Anything" MCP server available).
* SignalRGB 2.5.80 inventory (`REVERSE_ENGINEERING.md`), hardware inventory (`DEVICES.md`).
* Rust 1.99.1 + MSVC Build Tools installed (approved by the user).
* Corsair Bragi protocol implemented and validated on the K70 CORE (`PROTOCOLS.md`).
* Project renamed to freeRGB; private repo `github.com/leotrax3d/FreeRGB`.
* `freergbd`: render thread, reconnect with back-off, hot-plug rescan, JSON-RPC over a
  named pipe / Unix socket, clean release to hardware mode on exit.

## Working components [confirmed]
| Crate | Content |
|---|---|
| `freergb-core` | Rgb/Hsv, colour correction, LedLayout, Canvas, Frame |
| `freergb-device-api` | `HidTransport`, `RgbDevice`, `VirtualDevice`, `MockTransport` |
| `freergb-effects` | Solid, RainbowWave, Breathing, serializable `EffectSpec` |
| `freergb-hid` | hidapi discovery/transport, report descriptor parser |
| `freergb-protocols` | Corsair Bragi adapter, K70 CORE layout, adapter registry |
| `freergb-daemon` | engine, JSON-RPC API, IPC, hardware scan; binary `freergbd` |
| `freergb-cli` | binary `freergb`: `discover`, `demo-virtual`, `bragi …`, `ctl` |

## Tests
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` → 40 tests passing (Windows only; Linux not yet built).
Hardware results: `DEVICES.md#validation-log`.

## Open issues / questions
* K70 CORE frame rate ~22 FPS (every stream packet is acknowledged) – profile and optimise.
* `1b1c:1bf0` (no USB interface) – origin unknown.
* Full `key-walk` over all 107 keys not yet visually verified.
* `freergbd` is not installed as a service/autostart (needs user approval: system change).
* The SignalRGB app is currently stopped (its service still runs). Restart with
  `%LOCALAPPDATA%\VortxEngine\SignalRgbLauncher.exe`.

## Next tasks
1. GUI (`freergb-gui`, egui) talking only to the API.
2. Declarative device descriptions (TOML) as first plugin stage.
3. Analyse the iCUE LINK hub protocol (fans/AIO), read-only first.
4. GitHub Actions CI for Windows + Linux.
5. Profiles (persisted effect/brightness), autostart proposal.

## Entry points
`crates/freergb-daemon/src/main.rs`, `crates/freergb-protocols/src/corsair_bragi/device.rs`,
`crates/freergb-cli/src/main.rs`. SignalRGB plugins (analysis source):
`%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins`.
