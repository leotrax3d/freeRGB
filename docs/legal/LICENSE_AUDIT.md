# Lizenz- und Abhängigkeitsprüfung

**Keine pauschal festgestellte GPL-Verletzung; Lizenzkonformität des fertigen Produkts ist nicht belegt.** Nachgewiesen ist die bewusste Verwendung von GPL-Portierungen und die unvollständige Auslieferungs-/Fremdpaketprüfung (FR-007/010/011/015/018).

| Bereich | Tatsächlicher Befund | Pflichten und offene Prüfung |
|---|---|---|
| Eigene Core-Crates | Workspace MIT OR Apache-2.0, beide Texte | Fremdanteile bleiben gesondert; Copyright/NOTICE aus gewählter Lizenz beachten |
| 42 main-Hardwaremodule | Cargo GPL-2.0-or-later und COPYING vorhanden | Exakte Ursprungsdateien/Autoren, Änderungen und vollständiger korrespondierender Quellcode für Empfänger |
| Razer-Upstream | Gepinnter cpp-Header selbst gelesen: GPL-2.0-or-later | Bestätigt diesen Header, nicht automatisch alle 42 Familien |
| Mountain/ThingM-Branches | Eigene GPL-Manifeste und Provenance | Vor Integration eigenständige Artefakt-/Sourceprüfung |
| LibreHardwareMonitor v0.9.6 | MPL-2.0 Originaltext gelesen; gepinnter Download/Notices | Dateicopyleft, Sourcezugang; modifizierte Dateien und sämtliche mitgelieferten Teile prüfen |
| PawnIO/PawnIO.Modules | Manifest nennt Setup und eingebettete LGPL-2.1-Module | Setup-, Treiber- und Modullizenzen getrennt; Relink-/Sourcepflichten anhand konkreter Kombination prüfen |
| .NET Redistributable | Verifiziert gepinnter Microsoft-Download vorgesehen | Redistributable-Bedingungen sind keine MIT-Freigabe; tatsächlicher Vertrieb separat prüfen |
| GLib Rust-Backport | vendor/glib LICENSE und Backport-Herkunft; Integritätsskript bestanden | Rustbinding-Lizenz und native GLib/GTK-Laufzeitlizenzen nicht verwechseln; Linux-Dynamik/Weitergabe separat |
| Inter | Desktop-Lock: OFL-1.1 | Fontlizenz/Copyright in Distribution; vorgesehene Fontverwendung/Änderung prüfen |
| React/Radix/Lucide | Lockdeklarationen MIT/ISC | Texte/Attribution im erzeugten Produkt und Websitebundle prüfen |
| übrige Cargo-/npm-Pakete | Lockinventar erstellt | Nicht alle Original-Lizenztexte gelesen; NOASSERTION bleibt offen |
| FanControl | Offizielle LICENSE proprietär | Funktionsparität ist kein Recht zur Code-/Binärübernahme; keine FanControl-Codeübernahme nachgewiesen |
| SignalRGB | Historische Quelltexteinsicht dokumentiert | Funktionsfakten von Ausdruck/Tabellen unterscheiden; konkreter Vertrag/Rechteinhaber und Herkunft ungeklärt |

GPL kann mit MIT-Code kombiniert werden, ohne fremde GPL-Anteile in MIT umzubenennen. Apache-2.0 ist nicht ohne Weiteres mit GPL-2.0-only kompatibel; GPL-2.0-or-later und die MIT-Alternative können Spielräume eröffnen, ersetzen aber keine Prüfung jeder tatsächlichen Abhängigkeit. MPL-Dateicopyleft und LGPL-Link-/Quellcodeanforderungen sind ebenfalls nicht allein durch die Endung .dll oder einen Prozesswechsel erledigt. AGPL-Netzwerkpflichten sind nur bei tatsächlich entsprechender Komponente/Änderung relevant; eine solche Pflicht wurde hier nicht als Projektbefund nachgewiesen.

Hardwaremodule kommunizieren über einen dokumentierten JSON-/HID-Broker. Das ist ein tragfähiger Ansatz für Trennung, aber keine bindende juristische Feststellung unabhängiger Werke. Die FSF-FAQ ist als Position des Lizenzgebers gekennzeichnet. Portierung/Übersetzung beseitigt keine Lizenzpflicht.

Der Modulworkflow hat Sourcearchive und einen eigenen Build aus dem Archiv; sein ID-Guard lässt jedoch nur Razer zu. Kein `modules`-/`effects`-Release liegt unter den vier erfassten Releases. Es wurde daher kein konkretes ausgeliefertes Modulbinär-/Quellcodepaar geprüft. Ein privates Repository ist nicht per se GPL-widrig: maßgeblich ist ausreichender Source-/Rechtezugang für die konkreten Empfänger, nicht zwingend Publikation für jedermann. Bei allgemein zugänglichen Binärdateien darf der korrespondierende Quellcode nicht faktisch für deren Empfänger gesperrt sein.

Siehe `LICENSE_MATRIX.csv`, `SBOM.cdx.json`, `SBOM_LIMITATIONS.md`. Die SBOM ist absichtlich als unvollständig gekennzeichnet. Sie darf nicht als Freigabeliste oder Beleg tatsächlicher Binärinhalte verwendet werden. Ein vollständiger Lizenztextscan, Zielplattformauflösung und Kontrolle nativer Runtimebestandteile bleibt Releasevoraussetzung.
