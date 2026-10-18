# Testing

## Automated
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
2026-10-18 (Windows 11, Rust 1.99.1): 40 tests passing. Linux: not yet run.

Categories:
* **Unit:** core colours/layouts/canvas, effects, report descriptor parser, packet encoding.
* **Simulation:** `corsair_bragi::device::tests` (simulated K70 with mode/handle state),
  `engine::tests` (virtual devices, reconnect back-off), `api::tests`, `ipc::tests`
  (JSON-RPC over an in-memory duplex stream).
* **Hardware:** manual, below.

## Hardware tests – K70 CORE (manual)
Precondition: SignalRGB stopped (`Stop-Process -Name SignalRgbLauncher,SignalRgb`).
1. `freergb bragi probe` → pid `0x1bfd`, read mode.
2. `freergb bragi color #ff0000 --seconds 20` → fully red, then hardware mode.
3. `freergb bragi keys Esc=#ffffff "Left Ctrl=#ff0000" Fn=#ffff00 …` → only those keys lit.
4. `freergb bragi key-walk` → one key at a time; compare with printed names.
5. `freergb bragi effect rainbow --fps 60` → prints achieved FPS.

## Daemon end-to-end (manual)
1. `freergbd --virtual-device`
2. `freergb ctl devices.list` → K70 + virtual strip `Connected`.
3. `freergb ctl color.set '{"color":"#00ff00"}'`, `freergb ctl diagnostics.get`.
4. `freergb ctl daemon.shutdown` → process exits; `freergb bragi probe` shows mode 1.

Results: `DEVICES.md`.
