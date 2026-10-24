# Prompt: Restliche Mainboard-Arbeit (alles außer MSI)

Du arbeitest im Projekt **freeRGB**. Grundlage ist
`docs/promts/multi-fan-control-and-mainboard-rgb.md`. Lies davon nur die
Abschnitte „Effizienzregeln“, „Pflichtlektüre“ und „Status ‚normal‘“. Dieser
Prompt ersetzt die Aufteilung dort und legt fest, was **du** umsetzt.

**Nicht dein Teil:** Der MSI-Mystic-Light-Adapter wird parallel von einem
anderen Agenten gebaut. Fasse keine MSI-Dateien an.

## Arbeitsweise

- Eigener Branch von aktuellem `main`. Vor jedem Feature-Commit
  `git fetch origin main` und, falls der MSI-Branch inzwischen gemergt ist,
  `main` hereinmergen (kein Rebase).
- Gemeinsame Dateien (`crates/freergb-protocols/src/registry.rs`, `lib.rs`,
  `docs/MAINBOARD_RGB_DEVICES.md`) nur durch **Hinzufügen eigener Zeilen**
  ändern, nichts umsortieren oder umformatieren.
- Effizienz: keine Sub-Agenten, nur nötige Dateien lesen, ein kurzer Plan,
  gezielte Tests pro Schritt, volle Checks einmal am Ende
  (`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets -- -D warnings`,
  in `apps/desktop` `npm test` und `npm run build`).
- OpenRGB nur als Protokollreferenz (Commit
  `7182807a288a9e8107a84fbebc44239677f1ea24`), keine GPL-Code-Kopie in `crates/`.
  Eigene Rust-Implementierung aus Wire-Fakten wie `asus_aura.rs`, Quelllinks in
  der Doku.

## Reihenfolge

### 1. Gemeinsame Mainboard-RGB-Konfiguration (zuerst)

Prüfe zuerst, ob `main` oder der MSI-Branch (`git branch -r`) schon eine
herstellerübergreifende Header-Konfiguration hat. Wenn ja: übernehmen. Wenn
nein: baue sie herstellerneutral, damit MSI sie später nutzt:

- Pro Controller und Header: LED-Anzahl (0..512), Name, Preset (8/12/16/18 LEDs).
- Globaler Schalter „Mainboard-RGB verwalten“ (Standard: an).
- Gespeichert im Daemon-State, Zugriff nur über RPC; Editor in beiden GUIs.
- `--enable-asus-aura` und `--asus-argb-leds` bleiben als Override für Tests.
- Warnung, wenn Armoury Crate, MSI Center oder RGB Fusion laufen.

### 2. ASUS: alle Aura-USB-IDs

Allowlist um alle in `docs/MAINBOARD_RGB_DEVICES.md` gelisteten ASUS-IDs erweitern
(Mainboard und Addressable). Automatische Erkennung ohne Opt-in.
Mock-Tests pro ID-Gruppe.

### 3. Gigabyte RGB Fusion 2 USB

**Nicht neu bauen:** Auf Branch `module/gigabyte-rgbfusion2` existiert bereits
ein GPL-Modul (`modules/gigabyte-rgbfusion2/`) mit allen IDs (`048d:8297/8950/5702/5711`, …).
Prüfe es, bringe es über `modules/integration` nach `main` (falls noch nicht
geschehen) und schließe es an die gemeinsame Header-Konfiguration aus Schritt 1
an. Fehlende Mock-Tests ergänzen (ungültige Antworten → keine Writes, Fallback
beim Beenden). Prüfe vor Schritt 4 ebenso, ob es schon `module/asrock*`-Branches gibt.

### 4. ASRock Polychrome USB

`26ce:01a2` und `26ce:01a6`, gleiche Testabdeckung wie Gigabyte.

### 5. Mehrere Lüfter gleichzeitig

Wie Feature 1 im Hauptprompt: mehrere Reservierungen pro Worker, Lease pro
Kanal, Journal v2 (v1 lesbar und migriert), Recovery für alle Kanäle,
Sensorausfall betrifft nur den Kanal, Worker-Ausfall alle. Rust-Runtime pro
Kanal, gemeinsamer Transport mit Kanal-ID, max. 16 Kanäle. API additiv (Liste,
Start/Stopp pro Kanal, „alle stoppen“). Kurve↔Kanal persistent, Neustart nur bei
unveränderter Hardware-Identität. Sicherheitsgrenzen unverändert.

### 6. Moderne GUI für Lüfter

Wie Feature 2 im Hauptprompt: Kanalübersicht, Kurve/manueller Wert/Kalibrierung/
Start-Stopp pro Kanal, „Alle stoppen“, eine UAC-Bestätigung pro Sitzung,
Demo-Modus mit mehreren Kanälen, Vitest-Tests.

### 7. Abschluss

- Experimental-Kennzeichnungen und Opt-in-Pflicht für ASUS, Gigabyte, ASRock
  und die Lüftersteuerung entfernen; Sicherheitsmechanismen bleiben.
- Status-Spalte in `MAINBOARD_RGB_DEVICES.md` auf „Supported“ setzen
  (nur eigene Hersteller). Doku aktualisieren: `MAINBOARD_RGB.md`,
  `MAINBOARD_SUPPORT.md`, `COOLING_RUNTIME.md`, `FANCONTROL_PARITY.md`,
  `WORKLIST.md`, README.
- Abschnitt „Hardware-Validierung“: was real getestet ist, was nur gegen Mocks.
- Issue-Vorlage `.github/ISSUE_TEMPLATE/hardware-report.md` für Board-Rückmeldungen
  (Board, Controller-ID, Header, Ergebnis).
- Commits pro Schritt, am Ende pushen, Draft-PR mit Zusammenfassung,
  ungetesteter Hardware und Hinweis auf mögliche Konflikte mit dem MSI-PR.

Falls das Limit knapp wird: Schritte 1–5 haben Vorrang. Lieber einen Schritt
vollständig und getestet abschließen als mehrere halb.
