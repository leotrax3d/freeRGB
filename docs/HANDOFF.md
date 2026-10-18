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
`freergb-core`, `freergb-device-api` (VirtualDevice, MockTransport), `freergb-effects`
(Solid, RainbowWave, Breathing), `freergb-hid` (Discovery, Transport, Descriptor-Parser),
`freergb-protocols` (Bragi, Registry), CLI `freergb` (`discover`, `demo-virtual`,
`bragi probe|color|effect|key-walk|keys`).

## Tests
`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
`cargo test --workspace` (35 bestanden) – alle grün, nur Windows. Linux ungetestet.
Hardwaretests: siehe `docs/DEVICES.md#validierungsprotokoll`.

## Offene Fehler / Fragen
* Bildrate K70 nur ~22 FPS (Debug-Build) – Release messen, Drain/Quittungen optimieren.
* `1b1c:1bf0` ohne USB-Interface – Herkunft unklar.
* `key-walk` über alle 107 Tasten noch nicht visuell geprüft.
* Remote: privates Repo github.com/leotrax3d/FreeRGB (main).
* SignalRGB-App ist beendet (Dienst läuft). Neustart: `%LOCALAPPDATA%\VortxEngine\SignalRgbLauncher.exe`.

## Nächste Aufgaben
1. Daemon `freergbd`: Render-Loop (Canvas → Geräte), Gerätezustände, Reconnect.
2. Lokale API (Named Pipe / Unix-Socket, JSON-RPC), `docs/LOCAL_API.md` konkretisieren.
3. iCUE-LINK-Hub-Protokoll analysieren (Lüfter/AIO), zunächst read-only.
4. Linux-CI (GitHub Actions).

## Einstiegspunkte
`crates/freergb-protocols/src/corsair_bragi/device.rs`, `crates/freergb-cli/src/main.rs`,
SignalRGB-Plugins unter `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins`.
