# Prompt: Weitere OpenRGB-Controllerfamilien als freeRGB-Hardwaremodule portieren

Du arbeitest im Rust-Projekt **freeRGB** auf Basis des Branches
`feature/hardware-modules` (oder `main`, falls dieser dort bereits gemergt ist).
Lies zuerst vollständig: `docs/HARDWARE_MODULES.md`, `modules/razer-chroma/`
(Referenzmodul inkl. `PROVENANCE.md`, `source-workspace.toml`,
`module-info.json`), `crates/freergb-module-api/src/{lib,sdk}.rs`,
`scripts/openrgb/port-razer-devices.mjs` und `.github/workflows/modules.yml`.

## Ziel

Portiere weitere Controllerfamilien aus dem offiziellen OpenRGB-Projekt
(https://gitlab.com/CalcProgrammer1/OpenRGB) nach Rust – **jeweils als eigenes
Modul** unter `modules/<id>/` nach exakt dem Muster von `razer-chroma`. Ein
vollständiger Nachbau von OpenRGB ist nicht verlangt; arbeite Familie für
Familie ab, jede für sich vollständig, getestet und dokumentiert.

## Arbeitsumgebung (zuerst)

Der Ordner `<repo>` wird von anderen Sitzungen
gleichzeitig benutzt. Arbeite **nie** direkt darin und wechsle dort keinen
Branch. Lege einen eigenen Worktree an, z. B.
`git worktree add ../FreeRGB-modules -b module/<id> origin/feature/hardware-modules`,
und arbeite, baue und committe ausschließlich dort (eigener `target`-Ordner).
Prüfe vor jedem Commit mit `git branch --show-current`, dass du auf deinem
Modul-Branch bist. Pushe nur deine eigenen `module/*`-Branches.

## Verbindliche Regeln

1. **Upstream fixieren:** Verwende einen festen OpenRGB-Commit (Standard:
   `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, sonst begründet einen neueren)
   und nenne ihn in `PROVENANCE.md`, `module-info.json` und generierten Dateien.
2. **Lizenz pro Datei prüfen** (SPDX-Header jeder genutzten Datei inklusive
   Hilfsdateien). Module mit OpenRGB-Code sind `GPL-2.0-or-later`, enthalten
   `COPYING` und die Urheberhinweise; Änderungen werden gekennzeichnet. Der
   freeRGB-Kern (MIT OR Apache-2.0) darf **keinen** GPL-Code enthalten – nichts
   aus Modulen wandert in `crates/`. Kein SignalRGB-Code.
3. **Nur USB-HID-Familien** über die vorhandenen Transportaufrufe
   (`write`, `read`, `send_feature`, `get_feature`). Keine SMBus/I²C-,
   PCI-, Treiber- oder Bus-Scan-basierten Familien (Mainboard, RAM, GPU) –
   diese brauchen erst eine Erweiterung des Modulprotokolls und eine
   Produktentscheidung; liste sie nur als Kandidaten auf.
4. **Keine Doppelungen mit eingebauten Treibern:** freeRGB steuert Corsair
   K70 CORE (Bragi, 1b1c:1bfd/1bff), den iCUE LINK System Hub (1b1c:0c3f) und
   ASUS Aura USB (0b05:19af) bereits selbst. Diese Geräte in keinem Modul
   beanspruchen; bei Corsair-Familien sie in der Gerätetabelle ausschließen und
   das in `PROVENANCE.md` begründen. Prüfe zusätzlich `crates/freergb-protocols`.
5. **Gruppieren nach Protokoll**, nicht nach Produkt: ein Modul pro
   Controllerfamilie bzw. gemeinsamem Protokoll, Gerätetabellen per Skript
   reproduzierbar aus den Upstream-Dateien generieren (wie
   `port-razer-devices.mjs`), keine Treiber-Duplikate.
6. **Verhalten bewahren:** Erkennung (VID/PID/Interface/Usage Page/Usage,
   Varianten), Initialisierungssequenz, Paketformate, LED-Reihenfolge und
   -Namen, Timing/Sleeps, Antwortprüfung, Aufräumsequenz. Jede Abweichung
   steht in `PROVENANCE.md` unter „Changes“. Nur den Direct-Modus anbieten.
7. **Nicht Portiertes ehrlich melden:** Geräte/Codepfade, die du nicht
   vollständig überträgst, liefert `identify` als `supported: false` mit Grund.
   Keine Identität raten.
8. **Datenschutz:** keine Seriennummern lesen oder melden, keine Pfade.
9. **Tests (Pflicht je Modul):** byte-genaue Tests jedes Report-Builders gegen
   von Hand aus dem C++-Code abgeleitete Erwartungswerte, eine vollständige
   simulierte initialize/frame/shutdown-Sequenz für mindestens ein Gerät,
   Validierung aller unterstützten Layouts, Matching-Tests (falsches
   Interface/Usage wird abgelehnt). `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets -- -D warnings` und
   `cargo test --workspace` müssen grün sein.
10. **Packaging je Modul:** `Cargo.toml` (eigene Lizenz, `publish = false`),
   `src/main.rs` mit `--list-devices`, `COPYING`, `PROVENANCE.md` (Tabellen
   wie beim Razer-Modul: Upstream-Dateien → übernommenes Verhalten,
   Änderungen, Nicht-Portiertes, Teststatus „portiert / simuliert geprüft /
   auf Hardware bestätigt“), `source-workspace.toml`, `module-info.json`.
   Prüfe lokal, dass das Quellarchiv (`freergb-module-tool source …`)
   eigenständig baut.
11. **Nicht tun:** keine Releases veröffentlichen, keine Signaturschlüssel
    erzeugen oder einchecken, keine Änderungen am Modulprotokoll ohne
    Rückfrage, keine Hardwarezugriffe ohne ausdrückliche Zustimmung des
    Nutzers (Hardwaretests nur mit dem Nutzer, protokolliert in
    `docs/HARDWARE_MODULES.md`), bestehende Logoanimationen nicht ändern.

## Vorgehen

1. Inventar: Liste alle OpenRGB-Controllerverzeichnisse am fixierten Commit,
   klassifiziere nach Transport (HID / SMBus / anderes), Größe, gemeinsamen
   Abhängigkeiten (z. B. geteilte Hilfsklassen, Keyboard-Layout-Manager) und
   Lizenz. Priorität: reines USB-HID, viele Geräte pro Familie, wenige
   gemeinsame Abhängigkeiten. Testhardware gibt es derzeit **nur** von Corsair
   (K70 CORE, iCUE LINK – beide bereits eingebaut); andere Familien bleiben
   „simuliert geprüft“. Lege das Ergebnis als `docs/OPENRGB_PORTING.md` mit einer
   priorisierten Reihenfolge ab (Kriterien: reines HID, Anzahl Geräte,
   geringe Abhängigkeiten, vorhandene Testhardware laut `docs/DEVICES.md`).
2. Portiere die Familien in dieser Reihenfolge, je Familie ein Commit auf
   einem eigenen Branch `module/<id>`; nach jedem Commit pushen.
3. Ergänze im Workflow `modules.yml` nur, was nötig ist (z. B. Generator-Check
   für die neue Familie); der Ablauf bleibt pro Modul gleich.
4. Fasse am Ende zusammen: portierte Module, Gerätezahlen je Modul,
   bewusst ausgelassene Pfade, offene Hardwaretests, Blocker.

Gemeinsame Abhängigkeiten, die mehrere Familien brauchen (z. B. eine
Keyboard-Layout-Tabelle), werden als eigenes GPL-Crate unter `modules/shared-*`
portiert und von den Modulen per Pfad-Abhängigkeit genutzt – nie in den
MIT-Kern kopiert.
