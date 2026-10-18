# Architecture

```
freergb-gui ── freergb-client ──(JSON-RPC, local IPC)──▶ freergbd
freergb-cli ─────┐
freergb-daemon ──┼─ freergb-protocols ── freergb-device-api ── freergb-core
                 ├─ freergb-hid ──────── freergb-device-api
                 └─ freergb-effects ──── freergb-core
```

* **freergb-core** – Rgb/Hsv, colour correction, LedLayout, Canvas, Frame. No I/O.
* **freergb-device-api** – `HidTransport` (raw reports), `RgbDevice` (identify, connect,
  push_frame, status, disconnect), `VirtualDevice`, `MockTransport`.
* **freergb-protocols** – vendor protocols as pure packet functions plus a state machine
  generic over `HidTransport`; `registry` maps interfaces to adapters.
* **freergb-hid** – hidapi with native backends (Windows/Linux), report descriptor parser.
* **freergb-effects** – effects are pure functions of time → deterministic tests.
* **freergb-daemon** – `Engine` (effect → canvas → per-device sampling, colour
  correction, reconnect), JSON-RPC handler, IPC server, hardware scan.

## Runtime (`freergbd`)
* Render thread: `Engine::tick` renders at the target FPS and drops the newest frame into a
  per-device slot; hot-plug rescan every few seconds.
* One worker thread per device owns the device (connect, push, back-off retry,
  disconnect). A slow device skips stale frames (`dropped` counter) instead of slowing the
  others down.
* All hidapi calls in the process go through one global gate (`freergb-hid`): concurrent HID
  traffic during a K70 CORE transfer hung its vendor interface until re-plug (2026-10-18).
  The gate serialises single reports only, so devices still run at their own pace.
* Tokio runtime: IPC server; each request runs `api::handle_line` via `spawn_blocking`
  and briefly locks the engine.
* State: effect and brightness are saved to `%APPDATA%\freeRGB\state.json` (Linux:
  `$XDG_CONFIG_HOME/freergb/state.json`) on every change and restored on start (`--no-state` disables).
* Exit (Ctrl+C or `daemon.shutdown`): stop render thread, `Engine::shutdown` returns
  every device to hardware mode.

Principles: transport, protocol and device function are separate; unknown devices are
never written to; property writes go through an allowlist; `unsafe` is forbidden
workspace-wide. Decisions: `DECISIONS.md`.

## GUI (`freergb-gui`)
egui/eframe (glow backend). Contains no device or protocol code. A worker thread
polls `devices.list`, `diagnostics.get` and `effects.get` once per second and sends
queued commands (bursts from slider drags are coalesced per method). The window only
repaints when new state arrives, keeping idle CPU low.
