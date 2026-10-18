# Architecture

```
freergb-cli ─────┐            (later: freergb-gui → API only)
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
* Render thread: `Engine::tick` at the target FPS, hot-plug rescan every few seconds.
  HID I/O is blocking, so it lives on a plain thread.
* Tokio runtime: IPC server; each request runs `api::handle_line` via `spawn_blocking`
  and briefly locks the engine.
* Exit (Ctrl+C or `daemon.shutdown`): stop render thread, `Engine::shutdown` returns
  every device to hardware mode.

Principles: transport, protocol and device function are separate; unknown devices are
never written to; property writes go through an allowlist; `unsafe` is forbidden
workspace-wide. Decisions: `DECISIONS.md`.
