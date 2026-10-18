# Testing

## Automated
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
2026-10-18: 74 tests passing on Windows 11 (Rust 1.99.1). CI (`.github/workflows/ci.yml`) builds and tests on Windows and Ubuntu; Linux hardware access is untested.

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

## GUI smoke test (manual)
`freergbd --virtual-device --no-hardware`, then `freergb-gui`: window shows
"daemon connected" and the virtual strip; changing the effect via `freergb ctl` keeps
the GUI alive. 2026-10-18: passed on Windows (start + state change). Interactive
controls were not clicked by an automated test yet.

## iCUE LINK hub (manual, needs user approval – fans and pump are attached)
Precondition: SignalRGB stopped, no `freergbd` running.
1. `freergb link-probe --raw` – read-only, mode must stay 1.
2. `freergb link-probe --software-window` – children, RPM, temperatures; prints
   "hardware mode verified".
3. `freergb link-light-test --seconds 10` – fans red/green/blue, pump head white,
   LCD ring magenta; RPM watchdog; hardware mode verified.
4. Soak: `freergb link-light-test --seconds 900` – check `glitches` and RPM min/max.
5. `freergbd --enable-link-hub`, `freergb ctl color.set …`, `freergb ctl daemon.shutdown`,
   then `freergb link-probe` must show mode 1.
Results: `DEVICES.md`.
