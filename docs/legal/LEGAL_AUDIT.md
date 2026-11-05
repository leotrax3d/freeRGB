# freeRGB: Legal-, Sicherheits- und Haftungsaudit

**Ergebnis: NO-GO für eine uneingeschränkte Hardwareveröffentlichung des untersuchten Stands.** 14 Branches, 5 Tags und die Metadaten von 530 brancherreichbaren Commits erfasst. Baseline `main@fb1a0af8f762104e017f0b56341415ee609e5d40`, Datum 2026-11-05. 2087 eindeutige UTF-8-Dateiblobs mit Git-Hashprüfung. Risikoorientierte Tiefenprüfung kritischer Pfade; keine manuelle Vollprüfung jeder historischen Zeile, kein vollständiges Fremdpaket-Lizenzaudit und keine Hardware-/Binärzertifizierung.

## Wesentliche Befunde

Nach fehlgeschlagener GPU-/Linux-Firmware-Rückgabe können Wiederherstellungseinträge verschwinden. Journalfehler blockieren die Übernahme nicht. Feste Drehzahlregelung enthält nicht überall einen obligatorischen Temperatur-Notpfad, und ein Thread/Drop im Daemon garantiert keine Rückgabe bei hartem Prozess-/Treiberverlust. Der Wizard verspricht dennoch universelle Firmware-Rückgabe. Neue Coolingplugin-Branches zählen auch kurze Fail-safe-Transfers als Erfolg. Diese Codebefunde belegen offene Sicherheitsgrenzen, keinen eingetretenen Schaden.

Die OpenRGB-Module sind ausdrücklich GPL-2.0-or-later, als getrennte Programme angelegt und mit Quellarchivmechanismus ausgestattet. Es wurde kein pauschaler GPL-Verstoß oder bewiesenes SignalRGB-/FanControl-Plagiat festgestellt. Wesentliche Nachweise fehlen jedoch: vollständige Herkunft je Familie, rechtliche Einordnung der konkreten Modulkommunikation und sämtliche Pflichten im fertigen Installer-/Frontend-/Font-/Runtimepayload. Der Modulreleaseworkflow lässt nur Razer zu. Der Cloudclient zählt Downloads unter pseudonymer Identität, während Projekttexte Cloud/Telemetrie ausschließen; Verantwortlichkeit, Nutzerinformation und Aufbewahrung sind zu klären.

## Positive, tatsächlich vorhandene Maßnahmen

Ed25519-Katalog mit Domain Separation; Hash-/Größen-/Manifestprüfungen vor Modulstart; begrenzte IPC-/Reportgrößen; lokale IPC statt offenem TCP-Port; Windows geschützter Service/Worker mit Telemetrie, Lease und Journalreservierung; Werte-/Kurvenvalidierung und Floors; WASM ohne Imports/WASI mit Fuel-/Speicherlimits; Installerhash und erneute Verifikation vor UAC; bewusst redigierte Reports mit Vorschau; explizite Ausnahme zwingender Haftung im Wizard. GLib-Backport-Integritätsprüfung und drei lokale Testfälle positiv. Diese Schutzmaßnahmen wurden nicht als Beweis universeller Hardwaresicherheit gewertet.

## Die 15 Pflichtfragen

1. **Aktuell rechtlich vertretbar veröffentlichbar?** Keine uneingeschränkte Freigabe. Das hängt von Produktumfang, Empfängern, Rechten, Anbieterrolle und Behebung der hohen Befunde ab. Kein pauschales Publikationsverbot für Quelltext behauptet.
2. **Nachgewiesene Lizenz-/Urheberrechtsprobleme?** Nachgewiesen sind Drittcodeübernahmen/GPL-Pflichten und Lücken im Auslieferungs-/Herkunftsnachweis; keine hier bewiesene pauschale Rechtsverletzung. Cargo-/npm-Metadaten ersetzen keine Originaltexte.
3. **Wer könnte Ansprüche stellen?** OpenRGB-Dateiautoren bei tatsächlicher Pflichtverletzung; Font-/Crate-/Runtimeurheber bei fehlenden Pflichthinweisen; Nutzer/Eigentümer bei zurechenbaren Schäden; Betroffene/Aufsicht bei einschlägiger Datenverarbeitung. SignalRGB/FanControl nur bei konkret betroffener Rechtsposition, nicht allein wegen Funktionsähnlichkeit. Kein tatsächlicher Anspruch oder Verfahren gegen freeRGB belegt.
4. **Persönliche Haftung bei CPU/GPU/Mainboardschaden?** Möglich, insbesondere bei eigenem schuldhaftem Sorgfaltsverstoß und nachgewiesener Kausalität. Weder Fehler noch Schadenssumme allein reichen; Rechtsform/Vertrag/Privilegien konkret prüfen.
5. **Brandfolgen?** Hohe Sach-/Personenfolgeschäden und bei vollständigem Tatbestand strafrechtliche Prüfung möglich. Ein durch freeRGB ausgelöster Brand wurde nicht nachgewiesen und nicht als wahrscheinlich quantifiziert.
6. **Welche Disclaimer wirksam?** Keine abschließende Wirksamkeitsfreigabe. Der Wizard lässt zwingende Haftung ausdrücklich bestehen, enthält aber überzogene technische Zusagen. §§276 Abs.3/309 Nr.7 BGB und Spezialrecht begrenzen Freizeichnung; Einbeziehung/Vertragsmodell entscheidend.
7. **OpenRGB-/FanControl-Portierungsrisiken?** OpenRGB-Module sind bewusst abgeleitete GPL-Werke; Übersetzung wahrt Pflichten. Trennungs-/Sourcefrage konkret klären. FanControl selbst ist proprietär; LHM ist eigenständig MPL. Kein FanControl-Port im untersuchten Bestand nachgewiesen.
8. **Fehlende Sicherheitsmechanismen?** Durchgängige persistente unresolved-Recovery, transaktionaler Journal-Erfolg vor I/O, temperaturbezogener Notpfad für jeden Manualmodus und nachgewiesene Ausfallsicherheit je Backend; neue API3-Fail-safe-Counts vollständig prüfen.
9. **Dynamische Erweiterungen?** Signaturen sichern Herkunft, nicht harmloses Verhalten. Native Programme besitzen OS-Benutzerrechte; Prozessisolation ist keine Sandbox. WASM-Effekte haben engere tatsächliche Grenzen.
10. **Welche Gesetze gelten?** Urheber-/Lizenzrecht, BGB und bei entsprechenden Tatsachen DSGVO/TDDDG/DDG/UWG, Spezialprodukthaftung/CRA/Verbraucherrecht; keine pauschale Anwendbarkeit jeder Vorschrift. Datierte Matrix im Regulatorybericht.
11. **Vor öffentlicher Beta zwingend?** Hohe technische Befunde schließen, Sicherheitsangaben korrigieren, Quellen-/Notice-/Datenschutz-/Anbieternachweise und Freigabetests am konkreten Artefakt vervollständigen. Für API3 zusätzlich Branchbefund schließen.
12. **Was bleibt unvermeidbar?** Unbekannte Hardware-/Firmware-/Treiberzustände, falsche Verkabelung/Versorgung, OS-/Stromausfälle, verbleibende Softwarefehler und Rechtsauslegungs-/Beweisrisiken. Nicht jedes Restrisiko rechtfertigt dieselbe Maßnahme; Betriebsgrenzen definieren.
13. **Persönliches Risiko senken?** Fehler verhindern, sichere Gerätefreigabe, Release-/Incidentnachweise, passende Organisations-/Versicherungslösung und juristische Prüfung. Ein rote-Checkbox-Konzept ist kein Ersatz.
14. **Was muss ein Anwalt klären?** Konkrete Vertrags-/FOSS-/Geschäftsrolle, GPL-Werkgrenze und Herkunft, neue Haftungsreform/CRA, Datenschutz und Haftungstext/Versicherungsumfang, bei Bedarf Kennzeichen/Patente. Vollständige Fragen in LAWYER_REVIEW.md.
15. **Welche Belege fehlen?** Neue Rust-Regressionen/Plattformtests, Artefaktextraktion, vollständige Fremdlicense-/Advisoryprüfung, tatsächlicher Sourcezugang, Hardwareabnahmen und Betreiber-/Geschäfts-/Datenverarbeitungssachverhalt. Umfang transparent in REPOSITORY_AUDIT.md.

## Dokumente

* [Prüfumfang und Branchmatrix](REPOSITORY_AUDIT.md), [maschinenlesbarer Umfang](AUDIT_SCOPE.json), [CSV](BRANCH_MATRIX.csv)
* [Risikoregister](RISK_REGISTER.md), [JSON](RISK_REGISTER.json), [priorisierte Maßnahmen](LEGAL_ACTION_PLAN.md)
* [Lizenzen](LICENSE_AUDIT.md), [Drittcode](THIRD_PARTY_CODE.md), [Lizenzmatrix](LICENSE_MATRIX.csv), [SBOM](SBOM.cdx.json), [SBOM-Grenzen](SBOM_LIMITATIONS.md)
* [Produktsicherheit](PRODUCT_SAFETY.md), [Hardwarehaftung und Fälle A–F](HARDWARE_DAMAGE_LIABILITY.md), [persönliche Haftung](PERSONAL_LIABILITY.md)
* [Regulatorik](REGULATORY_COMPLIANCE.md), [IP/Patente](IP_AND_PATENTS.md), [Rechtsprechung](LEGAL_CASE_RESEARCH.md)
* [Primärquellen und Qualität](LEGAL_SOURCES.md), [Anwaltsfragen](LAWYER_REVIEW.md), [Releasebewertung](RELEASE_LEGAL_READINESS.md)

Dies ist eine technische/rechtliche Risikoanalyse, keine anwaltliche Rechtsberatung. Die Bewertung folgt den nachgewiesenen Grenzen; fehlende Befunde sind kein Sicherheits- oder Rechtsfreiheitsbeweis.
