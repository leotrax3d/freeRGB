# Prompt: Mehrere Lüfter gleichzeitig, moderne GUI und volle Mainboard-RGB-Integration

Du arbeitest im Projekt **freeRGB** (Rust-Daemon, Tauri/React-GUI unter
`apps/desktop`, Classic-GUI `crates/freergb-gui`, Windows-Mainboard-Helfer in C#
unter `providers/mainboard`). Ziel ist, **in einer einzigen Sitzung** drei
zusammenhängende Features fertigzustellen und aus dem Status „experimentell“ in
den normalen Produktstatus zu überführen. Es gibt **keine Testhardware**: Alles
wird gegen Mocks, synthetische Transporte und die öffentlich dokumentierten
Protokolle aus OpenRGB entwickelt.

## Effizienzregeln (wichtig – alles muss in ein Usage-Limit passen)

- **Nicht die ganze Doku lesen.** Lies nur die unten genannten Dateien, und
  davon zuerst Kopf und Abschnitte „Remaining“/„Open“. Nutze `grep` statt
  vollständiger Reads für große Dateien.
- **Keine Sub-Agenten**, keine breiten Explorationen, keine Umbauten außerhalb
  des Scopes. Bestehende Strukturen erweitern, nicht neu erfinden.
- Plane einmal kurz (max. 10 Punkte), dann umsetzen. Kein wiederholtes
  Neu-Analysieren bereits gelesener Dateien.
- Tests: pro Feature gezielte Unit-/Mock-Tests, dann **einmal** am Ende
  `cargo test --workspace --locked`, `cargo clippy --workspace -- -D warnings`
  und in `apps/desktop` `npm test` + `npm run build`. Nur bei Fehlern gezielt
  nachbessern.
- Commits pro Feature (3–4 Commits), am Ende einmal pushen.

## Pflichtlektüre (nur diese)

- `docs/COOLING_RUNTIME.md` (Abschnitt „Remaining acceptance“, „Support tiers“)
- `docs/MAINBOARD_CONTROL_BRIDGE.md`, `docs/MAINBOARD_NATIVE_SESSION.md` (grep nach
  „reservation“, „channel“, „session“)
- `docs/MAINBOARD_RGB.md`, `crates/freergb-protocols/src/asus_aura.rs`,
  `crates/freergb-protocols/src/registry.rs`
- `crates/freergb-daemon/src/cooling_runtime*`, `cooling_sessions*`, `hardware.rs`
- `providers/mainboard/NativeControl.cs`, `NativeControlSession.cs`,
  `NativeControlJournal.cs`, `NativeControlWorker.cs`
- `apps/desktop/src/features/cooling/` (besonders `LiveSession.tsx`, `FanCard.tsx`,
  `CurvesSection.tsx`, `ChannelBindings.tsx`)
- `crates/freergb-gui/src/bin/freergb-launcher.rs` (Opt-in-Flags)

## Feature 1: Mehrere Lüfter gleichzeitig steuern

Aktuell kann genau eine native Session/Reservierung aktiv sein.

1. **Worker/C#:** Mehrere gleichzeitige Reservierungen pro Worker, je Kanal eine
   eigene Lease mit eigenem Ablauf. Reservierung prozessübergreifend pro
   physischem Kanal (Chip + Control-ID) exklusiv; zweite Anforderung auf
   denselben Kanal wird abgelehnt. Journal-Format auf mehrere Kanäle erweitern
   (Version 2, Version 1 weiterhin lesbar und wird migriert). Recovery nach
   Absturz gibt **alle** im Journal stehenden Kanäle ans BIOS zurück.
2. **Watchdog:** Ausfall eines Sensors oder einer Lease betrifft nur den
   jeweiligen Kanal (→ 100 % bzw. Firmware-Rückgabe). Ausfall des Workers oder
   Transports setzt **alle** Kanäle zurück.
3. **Rust-Daemon:** `cooling_runtime` pro Kanal als eigene Task, gemeinsamer
   Transport mit Multiplexing (Kanal-ID in jeder Nachricht). Begrenzte Anzahl
   (z. B. max. 16 Kanäle), begrenzte Queues. Pumpen bleiben auf 100 %, Lüfter
   behalten die bestehenden Untergrenzen der Support-Tiers.
4. **API:** Bestehende Session-API additiv erweitern: Liste aktiver Sessions,
   Start/Stopp pro Kanal, „alle stoppen“. Bestehende Ein-Kanal-Aufrufe bleiben
   kompatibel.
5. **Persistenz:** Zuordnung Kurve ↔ Kanal speichern und beim Daemon-Start
   automatisch wieder aufnehmen, sofern Hardware-Identität (Chip, Control-ID,
   Binding aus `cooling-channel-bindings.json`) unverändert ist. Bei Abweichung
   nicht starten, sondern Status „needs attention“ anzeigen.
6. **Tests:** Mock-Worker mit mehreren Kanälen: parallele Kurven, Ausfall eines
   Kanals, Worker-Kill, Journal-Recovery mit mehreren Einträgen, Konflikt bei
   doppelter Reservierung, Migration Journal v1 → v2.

## Feature 2: Moderne GUI (apps/desktop) – volle Lüftersteuerung

Die moderne GUI muss alles können, was Classic kann, plus Mehrkanal:

- Übersicht aller Kanäle (`FanOverview`/`FanCard`): Status (aus / Kurve aktiv /
  Fehler / needs attention), aktuelle RPM, Duty, Temperaturquelle.
- Pro Kanal: Kurve zuweisen, starten/stoppen, manuellen Prozentwert setzen
  (innerhalb der Tier-Grenzen), Kalibrierung starten, Kanal-Binding bearbeiten.
- „Alle stoppen“-Button, gut sichtbar.
- UAC-/Bestätigungsablauf wie in Classic, aber einmal pro Sitzung statt pro Kanal.
- Nur über die vorhandene RPC/Client-Schicht, keine Hardwarelogik in der GUI.
- Demo-Modus mit synthetischen Mehrkanal-Daten, damit die Ansicht ohne Hardware
  funktioniert; Vitest-Tests für Zustandslogik.

## Feature 3: Volle RGB-Integration für Lüfter am Mainboard

Ziel: RGB-/ARGB-Lüfter an Mainboard-Headern der großen Hersteller werden
automatisch erkannt und laufen wie jedes andere Gerät (Effekte, Layout-Canvas,
Helligkeit, Profile).

1. **Geräteliste:** `docs/MAINBOARD_RGB_DEVICES.md` ist die Zielliste; Status-Spalte bei Umsetzung auf „Supported“ aktualisieren.
2. **ASUS Aura USB:** Allowlist auf alle in OpenRGB gelisteten ASUS-Aura-
   Mainboard-USB-IDs erweitern. Gen1/Gen2-Handling beibehalten.
3. **Weitere Hersteller (USB-HID):** MSI Mystic Light (USB, z. B. `1462:7Bxx`),
   Gigabyte RGB Fusion 2 USB (`048d:5702`/`048d:8297`), ASRock Polychrome USB.
   Wo nur SMBus/I²C existiert (ältere Boards), **nicht** umsetzen, sondern in der
   Doku als „nicht unterstützt“ führen.
4. **OpenRGB als Referenz:** Nutze OpenRGB (festen Commit nennen) nur als
   Protokoll-Referenz. **Lizenz beachten:** Der Kern ist MIT OR Apache-2.0,
   OpenRGB ist GPL-2.0. Entweder unabhängige Rust-Implementierung aus
   dokumentierten Wire-Fakten (wie bei `asus_aura.rs`, mit Quelllinks in der Doku)
   oder ein GPL-Modul unter `modules/` nach dem Muster von `razer-chroma`.
   Kein kopierter C++-Code im Kern.
5. **Konfiguration in der GUI statt Launcher-Flag:** `--enable-asus-aura` und
   `--asus-argb-leds` ersetzen durch Geräteeinstellungen in beiden GUIs:
   LED-Anzahl pro ARGB-Header, Header-Namen, optional Lüfter-Presets (z. B. 8,
   12, 16, 18 LEDs). Gespeichert im Daemon-State, über RPC. Flags bleiben als
   Override für Tests erhalten.
6. **Automatische Aktivierung:** Unterstützte Controller werden beim Start
   normal erkannt und geöffnet (keine Opt-in-Pflicht mehr). Ein globaler Schalter
   „Mainboard-RGB verwalten“ (Standard: an) und Erkennung laufender
   Hersteller-Software (Armoury Crate, MSI Center, RGB Fusion), dann Warnung statt
   stillem Konflikt.
7. **Tests:** Mock-Transport pro Hersteller: Erkennung, Header-Tabellen,
   Paketaufbau/Chunking, Fallback beim Beenden, ungültige Antworten → keine Writes.

## Status „normal“ statt „experimentell“

- Entferne Experimental-Kennzeichnungen in GUI, CLI-Hilfe und Doku für die drei
  Features; die Lüftersteuerung wird im modernen GUI regulär angeboten (Windows,
  sobald die Mainboard-Komponenten installiert sind).
- **Sicherheitsmechanismen bleiben vollständig aktiv:** Watchdog, Lease-Ablauf,
  Journal-Recovery, Untergrenzen, Pumpen nur 100 %, Rückgabe ans BIOS. Diese
  dürfen für den Statuswechsel nicht gelockert werden.
- Doku (`MAINBOARD_RGB.md`, `MAINBOARD_SUPPORT.md`, `COOLING_RUNTIME.md`,
  `FANCONTROL_PARITY.md`, `WORKLIST.md`, README-Features) aktualisieren. Ehrlich
  bleiben: ein kurzer Abschnitt „Hardware-Validierung“, der festhält, welche
  Boards/Controller real getestet sind und welche nur gegen Mocks; Nutzer werden
  um Rückmeldungen gebeten (Issue-Vorlage für Hardware-Reports).

## Abschluss

- Alle Checks grün (siehe Effizienzregeln), Commits pushen, Draft-PR mit kurzer
  Zusammenfassung pro Feature, Liste der ungetesteten Hardware und der
  verwendeten OpenRGB-Referenzen (Commit + Dateien).
