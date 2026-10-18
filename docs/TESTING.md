# Tests

## Automatisch
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Stand 2026-10-18 (Windows, Rust 1.99.1): 35 Tests grün. Linux: noch nicht ausgeführt.

Kategorien: Unit (core, effects, descriptor), Simulation (`corsair_bragi::device::tests`
mit simuliertem K70 inkl. Modus/Handle-Zustand), Hardware (manuell, unten).

## Hardwaretests K70 CORE (manuell)
Voraussetzung: SignalRGB beendet (`Stop-Process -Name SignalRgbLauncher,SignalRgb`).
1. `signal-rs bragi probe` → pid `0x1bfd`, Modus lesen.
2. `signal-rs bragi color #ff0000 --seconds 20` → komplett rot, danach Hardware-Modus.
3. `signal-rs bragi keys Esc=#ffffff "Left Ctrl=#ff0000" Fn=#ffff00 …` → nur genannte Tasten.
4. `signal-rs bragi key-walk` → jede Taste einzeln, Namen in Ausgabe vergleichen.
5. `signal-rs bragi effect rainbow --fps 60` → FPS-Ausgabe.
Ergebnisse: `docs/DEVICES.md`.
