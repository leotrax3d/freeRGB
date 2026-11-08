# Prompt: Vollständige OpenRGB-Geräteabdeckung in freeRGB (HID-Rest + SMBus/I²C/GPU + SDK-Netzwerk)

Du arbeitest im Rust-Projekt **freeRGB** (Repo `leotrax3d/FreeRGB`, Basis:
`origin/feature/parity-expansion`, falls schon gemergt `origin/main`).
Verbindliches Endziel des Projekts: **freeRGB unterstützt 100 % der Geräte, die
OpenRGB unterstützt**, mit allen RGB-Funktionen, über den Rust-Hintergrunddienst
`freergbd`. Priorisieren ist erlaubt, dauerhaftes Ausschließen nicht.

## 0. Arbeitsumgebung (zwingend zuerst)

* `<repo>` wird von anderen Sitzungen gleichzeitig
  benutzt. Arbeite **nie** darin, wechsle dort keinen Branch. Lege einen eigenen
  Worktree an:
  `git worktree add ../FreeRGB-coverage -b coverage/<arbeitspaket> origin/feature/parity-expansion`
  und baue/committe nur dort (eigenes `target/`).
* Vor jedem Commit `git branch --show-current` prüfen. Pro Familie ein Commit,
  nach jedem Commit den eigenen Branch pushen. Nie `main` pushen, nie Releases
  veröffentlichen, nie Signaturschlüssel erzeugen oder einchecken.
* Repo-Sprache Englisch (Code, Kommentare, Docs, Commits). Projektname immer
  „freeRGB“. Commits enden mit
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
* Lange Builds/Tests (> 3 min) im Hintergrund starten und währenddessen weiterarbeiten.

## 1. Pflichtlektüre vor jeder Änderung

Lies vollständig und baue darauf auf – nichts neu erfinden, was existiert:

* `docs/PARITY_EXPANSION_PLAN.md`, `docs/OPENRGB_PORTING.md` (Inventar +
  Port-Ledger), `docs/HARDWARE_MODULES.md` (Modulprotokoll, GPL-Trennung,
  Packaging), `docs/DECISIONS.md` (v. a. #11 HID-Gate), `docs/DEVICES.md`,
  `docs/MAINBOARD_RGB.md`, `docs/MAINBOARD_SUPPORT.md`, `docs/SECURITY.md`.
* `docs/promts/port-openrgb-controller-families.md` – die bisherigen
  Portierungsregeln gelten weiter (Regeln 1–11 dort), soweit dieser Prompt sie
  nicht ausdrücklich erweitert.
* Referenzmodule: `modules/razer-chroma/` (Muster), dazu ein kleines neueres
  wie `modules/das-keyboard/`; `crates/freergb-module-api/src/{lib,sdk}.rs`,
  `crates/freergb-modules/`, `crates/freergb-daemon/src/modules.rs`,
  `crates/freergb-protocols/src/registry.rs` (eingebaute Treiber),
  `scripts/openrgb/inventory.mjs`, ein `scripts/openrgb/port-*-devices.mjs`,
  `.github/workflows/modules.yml`.
* Falls vorhanden: `scripts/openrgb/compat-matrix.mjs` und
  `docs/OPENRGB_COMPATIBILITY.md` (Gerätematrix). Wenn nicht vorhanden, ist
  Arbeitspaket A Schritt 1 diese Matrix.

Stand heute: 42 Module unter `modules/` decken 42 der 121 HID-Familien ab
(fast alle nur Direct-Modus, nur simuliert geprüft). Hardware-verifiziert sind
nur K70 CORE und iCUE LINK (eingebaut). Nicht-HID-Transporte fehlen komplett.

## 2. Upstream und Lizenz

* OpenRGB: https://gitlab.com/CalcProgrammer1/OpenRGB, fixierter Commit
  `bbc77470a7cba56af60989e54ba910d5ac66a3ac`. Ein neuerer Commit nur
  begründet und dann für das ganze Arbeitspaket einheitlich.
* SPDX-Header **jeder** genutzten Datei prüfen. OpenRGB-Code ⇒ Modul ist
  `GPL-2.0-or-later` (bzw. `GPL-2.0-only`, wenn eine Datei das verlangt – dann
  das ganze Modul), mit `COPYING`, Urheberhinweisen, `PROVENANCE.md`.
  **Kein GPL-Code in `crates/`** (Kern ist MIT OR Apache-2.0). Gemeinsame
  GPL-Hilfen nur als `modules/shared-*`-Crate. Kein SignalRGB-, iCUE-,
  Armoury-Crate- oder anderer proprietärer Code; keine Dekompilierung.
* Unklare oder fehlende Lizenz ⇒ Familie als „blocked: license“ in der Matrix
  führen und **Rückfrage** stellen, nicht portieren.

## 3. Arbeitspaket A – restliche USB-HID-Familien (ca. 79)

Ziel: jede noch nicht portierte HID-Familie aus `docs/OPENRGB_PORTING.md` als
eigenes Modul `modules/<id>/`, exakt nach dem bestehenden Muster.

1. Erzeuge (falls noch nicht vorhanden) die Restliste: alle Familien des
   Inventars minus vorhandene Module minus eingebaute Treiber. Prüfe auf
   bestehenden Code (auch unmergte Branches `module/*`, `modules/integration`)
   – **keine Doppel-Implementierungen**.
2. Reihenfolge: zuerst Familien mit mehreren Geräten und gemeinsamer
   Infrastruktur, etwa `QMKController` (OpenRGB- und VialRGB-Protokoll,
   dynamische Detektoren!), `WootingKeyboardController`
   (eigener `REGISTER_WOOTING_DETECTOR`), `MountainKeyboardController`,
   `RealtekBridgeController`, `ViewSonicController`, Alienware (3 Familien),
   `AOC*`, `Clevo*`, `Cougar`, `Ducky`, `Holtek`, `NZXTKraken`, `HyperXMicrophone*`
   (`REGISTER_HID_WRAPPED_DETECTOR*`), dann die Einzelgerät-Familien.
   `ASRockPolychromeUSBController`, `CorsairICueLinkController` und
   `LuxaforController` sind bereits abgedeckt – nur in der Matrix verknüpfen.
3. Gemeinsame Abhängigkeiten wie `KeyboardLayoutManager` einmal als
   `modules/shared-keyboard-layout` portieren und wiederverwenden.
4. **Nicht nur Direct-Modus:** Wo die Familie Hardware-Effekte/-Modi,
   Helligkeit, Geschwindigkeit, Farben pro Modus oder Speichern im Gerät hat,
   portiere sie ebenfalls, sofern das Modulprotokoll das abbilden kann. Wenn
   das Protokoll dafür fehlt: zuerst einen Vorschlag für eine **additive,
   versionierte** Protokollerweiterung (z. B. `modes`-Capability in
   `identify`, `set_mode`-Aufruf) in `docs/HARDWARE_MODULES.md` schreiben und
   **Rückfrage stellen**, bevor du das Protokoll änderst.
5. Pflichttests je Modul (wie bisher): byte-genaue Report-Tests gegen aus dem
   C++-Code abgeleitete Erwartungswerte, simulierte
   initialize/frame/shutdown-Sequenz, Layout-Validierung, Negativ-Matching.
   Generator-Check in `modules.yml` ergänzen.

## 4. Arbeitspaket B – SMBus/I²C (RAM, Mainboard-SMBus, GPU über I²C/PCI)

Das ist **sicherheitskritisch** (falsche SMBus-Schreibzugriffe können RAM-SPD
oder Hardware beschädigen; OpenRGB-Historie: zerstörte Gigabyte/ASUS-RAM-SPDs).
Deshalb erst Design, dann Freigabe, dann Code.

Upstream-Umfang am fixierten Commit: `REGISTER_I2C_PCI_DETECTOR` (~905, fast
alles GPUs), `REGISTER_I2C_DRAM_DETECTOR` (10), `REGISTER_I2C_DETECTOR` (9),
`REGISTER_I2C_BUS_DETECTOR`, plus SMBus-Mainboard-Familien
(ASUS Aura SMBus/ENE, ASRock SMBus, Gigabyte RGB Fusion SMBus, MSI GPU …).

1. **Design-Dokument** `docs/SMBUS_I2C_TRANSPORT.md` (ohne Code-Änderungen):
   * Wie bekommt `freergbd` auf Windows I²C/SMBus-Zugriff ohne
     Zwangstreiber? Optionen bewerten: PawnIO (bereits optional für LHM im
     Projekt, siehe `docs/MAINBOARD_SUPPORT.md`), WinRing0 (Sicherheitsproblem,
     verwundbarer Treiber – nicht empfehlen), NVIDIA NvAPI I²C, AMD ADL I²C,
     Linux `i2c-dev`. Lizenz und Vertrieb jeder Option.
   * Brokered Transport wie bei HID: Das Modul öffnet nie den Bus; der Daemon
     führt `smbus_read_byte_data`/`write_byte_data`/`block`-Aufrufe aus,
     serialisiert pro Bus (globales Gate wie DECISIONS #11).
   * **Allowlist-Pflicht**: Der Daemon erlaubt einem Modul nur die
     Adressen/Register, die dessen signiertes `module.json` deklariert;
     Schreibzugriffe auf SPD-Adressen `0x50–0x57` (und Page-Select `0x36/0x37`)
     sind grundsätzlich verboten; Erkennung nur über die Lesesequenzen, die
     OpenRGB nutzt.
   * Opt-in pro Gerät in der GUI mit Warnhinweis, Default aus; Abbruch und
     Wiederherstellung bei Fehlern; Logging.
   * Protokollerweiterung (additiv, versioniert) für `smbus_*`/`i2c_*`-Aufrufe
     und PCI-ID-Matching (ven/dev/subven/subdev) im Modul-Manifest.
2. **Rückfrage an den Nutzer** mit Empfehlung, bevor du Treiber-, Protokoll-
   oder Sicherheitsentscheidungen umsetzt. Keine Treiber installieren, keine
   Hardwarezugriffe ohne ausdrückliche Zustimmung (Testhardware: Corsair
   Vengeance RGB DDR5 an SMBus 0x18/0x1A, NVIDIA RTX 5060 Ti (Gigabyte),
   AMD RX 9060 XT – siehe `docs/DEVICES.md`; RAM ist bewusst „deferred (risk)“).
3. Nach Freigabe: Implementierung im Daemon (MIT-Kern, kein GPL) mit
   **simuliertem Bus** (Mock-SMBus mit Registermodell) für Tests; dann die
   GPU-Familien (große Detektorlisten per Generator wie bei HID) und RAM-
   Familien als GPL-Module. Jede Familie: byte-genaue Transaktionstests gegen
   den Mock, inkl. Test, dass verbotene Adressen abgewiesen werden.

## 5. Arbeitspaket C – OpenRGB SDK-Netzwerkprotokoll + übrige Transporte

1. **SDK-Server in `freergbd`**: Implementiere das OpenRGB-SDK-Protokoll
   (TCP, Standardport 6742, Header `ORGB` + Device-Index + Packet-ID +
   Länge; Protokollversionen bis zur am fixierten Commit aktuellen). Die
   Protokollbeschreibung aus OpenRGBs Doku/`NetworkProtocol.h` als Spezifikation
   nutzen und **clean-room in MIT-Rust** nachimplementieren (keinen GPL-Code
   kopieren; im Zweifel Rückfrage). Damit funktionieren vorhandene SDK-Clients
   (Artemis, Home Assistant, Spiele-Integrationen) mit freeRGB. Standard: nur
   `127.0.0.1`, Netzwerkfreigabe nur per ausdrücklicher Einstellung.
2. **SDK-Client** als optionale Quelle: entfernte OpenRGB-Instanzen als Geräte
   einbinden.
3. Übrige OpenRGB-Transporte inventarisieren und je eine Design-Notiz +
   Matrix-Status anlegen: `REGISTER_USB_SERIAL_DETECTOR` (z. B. BlinkyTape,
   HYTE CNVS – serielle Ports brokern), `REGISTER_DETECTOR`-Familien mit
   Netzwerk (Philips Hue, Nanoleaf, Yeelight, LIFX, Govee, E1.31, WLED/DDP),
   libusb-only-Geräte, Linux-LED-Klasse, Windows-Gerätetreiber-basierte
   Familien. Netzwerk-Familien können als MIT-Clean-Room oder GPL-Modul
   kommen – je Familie begründen.

## 6. Kompatibilitätsmatrix und Synchronisierung (für alle Pakete)

* Jede Statusänderung aktualisiert die automatisch erzeugte Matrix
  (`scripts/openrgb/compat-matrix.mjs` → `docs/OPENRGB_COMPATIBILITY.md` +
  `docs/openrgb-compatibility.json`). Pro Gerät: OpenRGB-Name, Familie,
  Transport, IDs, freeRGB-Implementierung, Funktionen, Status.
* Statusstufen streng trennen: `not-implemented`, `blocked:<grund>`,
  `implemented` (Code + Tests, **simuliert geprüft**), `hardware-verified`
  (nur mit protokolliertem Nutzertest in `docs/DEVICES.md`). Nie hochstufen
  ohne Nachweis.
* Upstream-Drift: ein Skript/CI-Job vergleicht den fixierten Commit mit
  OpenRGB-`master` und listet neue/geänderte/entfernte Detektoren und
  geänderte Controller-Dateien; Ergebnis als Bericht, kein automatisches
  Portieren.

## 7. Qualitätsregeln

* `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` grün; jedes Modul-Quellarchiv baut eigenständig.
* Keine Platzhalter: Geräte/Codepfade, die nicht vollständig portiert sind,
  liefert `identify` als `supported: false` mit Grund und die Matrix zeigt sie
  als offen.
* Keine Seriennummern/Pfade lesen oder melden. Keine Hardwarezugriffe ohne
  Zustimmung. Eingebaute Treiber (K70 CORE 1b1c:1bfd/1bff, iCUE LINK
  1b1c:0c3f/0c4e, ASUS Aura USB, Gigabyte Fusion 2 USB, ASRock Polychrome USB –
  siehe `registry.rs`) in keinem Modul beanspruchen.

## 8. Rückfragen (Pflicht, nicht selbst entscheiden)

Lizenzunklarheiten; jede Änderung am Modulprotokoll; Treiberwahl/-vertrieb für
SMBus/I²C; erste reale SMBus-/GPU-Schreibzugriffe; Öffnen des SDK-Servers fürs
Netzwerk; Hardwaretests.

## 9. Abschluss je Sitzung

Bericht (Deutsch an den Nutzer, Docs auf Englisch): portierte Familien mit
Gerätezahlen, Matrix-Kennzahlen vorher/nachher (implemented / verified / offen
je Transport), bewusst ausgelassene Pfade, offene Rückfragen, Branch-Namen.
Ehrlich bleiben: „portiert und simuliert geprüft“ ist nicht „funktioniert“.
