# HANDOFF – aktueller Stand (2026-10-18)

## Phase
Roadmap-Phasen 1–7 abgeschlossen: erste reale RGB-Steuerung (K70 CORE) validiert.
Nächste Phase: 8 (Hintergrunddienst + lokale API).

## Erledigt
* Werkzeug-/Umgebungsinventar (kein „Reverse Engineer Anything“-MCP vorhanden).
* SignalRGB 2.5.80 inventarisiert (`docs/REVERSE_ENGINEERING.md`).
* Hardware inventarisiert (`docs/DEVICES.md`).
* Rust 1.99.1 + MSVC Build Tools installiert (Freigabe des Nutzers).
* Workspace mit 6 Crates, 35 Unit-/Simulationstests.
* Bragi-Protokoll implementiert, an K70 CORE validiert (`docs/PROTOCOLS.md`).

## Funktionierende Komponenten [B]
`signal-core`, `signal-device-api` (VirtualDevice, MockTransport), `signal-effects`
(Solid, RainbowWave, Breathing), `signal-hid` (Discovery, Transport, Descriptor-Parser),
`signal-protocols` (Bragi, Registry), CLI `signal-rs` (`discover`, `demo-virtual`,
`bragi probe|color|effect|key-walk|keys`).

## Tests
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (35 bestanden) – alle grün, nur Windows. Linux ungetestet.
Hardwaretests: siehe `docs/DEVICES.md#validierungsprotokoll`.

## Offene Fehler / Fragen
* Bildrate K70 nur ~22 FPS (Debug-Build) – Release messen, Drain/Quittungen optimieren.
* `1b1c:1bf0` ohne USB-Interface – Herkunft unklar.
* `key-walk` über alle 107 Tasten noch nicht visuell geprüft.
* Kein Git-Remote: `gh` nicht angemeldet → Nutzer muss `gh auth login` ausführen.
* SignalRGB-App ist beendet (Dienst läuft). Neustart: `%LOCALAPPDATA%\VortxEngine\SignalRgbLauncher.exe`.

## Nächste Aufgaben
1. Daemon `signal-rsd`: Render-Loop (Canvas → Geräte), Gerätezustände, Reconnect.
2. Lokale API (Named Pipe / Unix-Socket, JSON-RPC), `docs/LOCAL_API.md` konkretisieren.
3. iCUE-LINK-Hub-Protokoll analysieren (Lüfter/AIO), zunächst read-only.
4. Linux-CI (GitHub Actions) sobald Remote existiert.

## Einstiegspunkte
`crates/signal-protocols/src/corsair_bragi/device.rs`, `crates/signal-cli/src/main.rs`,
SignalRGB-Plugins unter `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins`.
