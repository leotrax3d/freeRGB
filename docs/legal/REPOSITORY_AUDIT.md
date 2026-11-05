# Repositoryprüfung und Beweisgrenzen

Prüfdatum 2026-11-05; Baseline `main@fb1a0af8f762104e017f0b56341415ee609e5d40`. Zugriff über authentifizierte GitHub-Leseoperationen. Ein normaler Git-Clone scheiterte an fehlenden lokalen Git-Zugangsdaten. Kein Branch wurde zusammengeführt, keine Produktfunktion korrigiert, kein Installer gestartet und keine Hardware angesprochen.

## Tatsächlich erfasste Stände

| Referenz | Commit | Dateien | UTF-8 geladen | Neu/geändert ggü. main | Nicht vorhanden ggü. main |
| --- | --- | --- | --- | --- | --- |
| backup/modules-integration-remote | 9c4377de39e95264c18388e3c8f4e15a7c793633 | 877 | 838 | 362 | 449 |
| backup/modules-integration-wip | fe2196accc8369830465f035c9d29253596439e0 | 606 | 567 | 202 | 700 |
| backup/tauri-gui | b62cdfd05d2955e73601eab2778d5b86cea96a21 | 282 | 263 | 120 | 1025 |
| backup/website-live-demo | f6fcb72a3d54a41ae058c3cc04f0db0e4755476c | 479 | 440 | 180 | 834 |
| claude/adoring-hopper-p5zi8e | 7183645d3fcfa7351264e9c6b59cef7506797191 | 1131 | 1051 | 91 | 178 |
| cooling/controller-features | 08b09d31f293907e926df1e9cdafce40bb95b5b7 | 1302 | 1222 | 1 | 0 |
| cooling/gui-parity | 8295132be77810e782012528205624ecc930becc | 1335 | 1252 | 80 | 0 |
| cooling/plugins | 26588ad52d16fff0cdaeeee131ec877bcee96b05 | 1321 | 1241 | 71 | 3 |
| cooling/sensors-gpu | d2b875863f7fac63d5d5d7e6ac1289a2746cadf5 | 1299 | 1219 | 23 | 3 |
| cooling/service-profiles | 2716c68e7cf5dcbbfe0028b760f980824086e111 | 1328 | 1248 | 67 | 0 |
| main | fb1a0af8f762104e017f0b56341415ee609e5d40 | 1302 | 1222 | 0 | 0 |
| module/mountain-keyboard | c5163bcda87ad66db814d87c0221ccf6eea17887 | 681 | 642 | 171 | 636 |
| module/thingm-blink | 3851df95ccba65789262a9c17610c7503a119621 | 496 | 457 | 175 | 822 |
| modules/integration | c26b9d191e75d90265e99d64baa4f74290e4a6e9 | 934 | 895 | 403 | 395 |
| refs/tags/prerelease-broad-fan-support-1 | bf9cf96878331977ad16a825badb5f76a53b2444 | 430 | 391 | 129 | 878 |
| refs/tags/v0.1.0-beta.1 | eb1184a54bdacaebcc18d0f1e92a87712f099edd | 526 | 484 | 132 | 781 |
| refs/tags/v0.1.0-beta.2 | 227214138b13de21bb57a52d737b41278f0e1622 | 1120 | 1040 | 97 | 189 |
| refs/tags/v0.1.0-beta.3 | 08ae89edd9df55e9c957977645037931546eadc9 | 1142 | 1062 | 78 | 167 |
| refs/tags/v0.1.0-beta.4 | 555a9f7d27186ae480bf19df296c34207401e5e8 | 1143 | 1063 | 71 | 166 |

Alle Bäume waren nicht abgeschnitten. 2087 eindeutige UTF-8-Blobs wurden lokal geladen und gegen den Git-Blob-SHA-1 geprüft: **0 Abweichungen**. Bilder/Icons/WASM-/Installerbinärdateien und sieben historische OneNote-Verzeichnisdateien wurden nicht inhaltlich geladen. Eine nicht UTF-8 lesbare `docs/EFFECT_COLLECTION.md` bleibt ebenfalls offen. Der vorübergehend fehlgeschlagene SteelSeries-Testdateiabruf wurde erfolgreich wiederholt. `AUDIT_SCOPE.json` führt jede fehlende Datei je Referenz auf; `FILE_COVERAGE.csv` trennt Abruf/Hashprüfung und mechanischen Scan. Diese Indikatoren sind **kein manuelles Review jeder Zeile**.

## Historie

530 unterschiedliche Commits aller 14 Branches erfasst, sämtliche Elternverweise innerhalb dieser Menge vorhanden. Alle fünf Tagziele auf Commit-SHAs aufgelöst; siehe JSON/Branchmatrix. Die Erreichbarkeit der Tagziele ist getrennt nachprüfbar. Unreachable/deleted Objekte und nicht vorhandene Remote-Refs sind nicht zugänglich. Metadaten und ausgewählte Herkunfts-/Sicherheitsänderungen wurden geprüft, insbesondere `565a110f6d9ed33b131afdfa864435ed937076ab` (Einführung Razer/GPL-Module). **Nicht alle 530 historischen Diffs und nicht jede historische Dateiversion wurden vollständig gelesen.**

## Vertiefte technische Pfade

* GUI-Onboarding → safety.acknowledge/require_safety → API-Assignment → hwmon/GPU-Regelung → sysfs/NVML; Rückgabe und Journale bis zur Löschentscheidung verfolgt.
* Windows-Session → authentifizierter Worker → JournaledNativeControl → NativeControl; Telemetrie-/Ticket-/Leasezustände und Notfallrückgabe statisch geprüft. Geschützte Installation und Testdateien anhand der tatsächlichen Quellen mitberücksichtigt, keine SCM-/Kerneltests hier.
* Hardwaremodul → Ed25519-Katalog → Hash-/Manifest-/Paketprüfung → nativer Prozess → Broker/HID; GPL-Quellarchiv-Workflow und Lizenzgrenze geprüft.
* Marketplace → WASM-Interpreter, Kraftstoff-/Speicherlimits und Verbot von Imports; Rating-/Download-/Reportpfad bis zum Appwrite-Datenmodell geprüft.
* Update → GitHub-Metadaten/Assethash → exklusives Staging → erneute Dateiprüfung → ShellExecute runas; Installer-/Notice-Einbindung geprüft.
* Nebenbranches: API-3-Coolinghost und Fail-safe-Transfers, Rust-Service-Supervisor und native Abhängigkeiten; Gerätebranches anhand Herkunft/Manifeste/Abweichungen.

## Branchspezifische Ergebnisse

* `cooling/controller-features` unterscheidet sich von main nur in einer Datei; Gleichheit des übrigen Baums geprüft.
* `cooling/sensors-gpu` hat 23 neue/geänderte Dateien; die von main geerbten hwmon-/GPU-Befunde werden durch Sensorquellenänderungen nicht automatisch behoben.
* `cooling/plugins`, `cooling/service-profiles`, `cooling/gui-parity` führen zusätzliche API-3-/Servicepfade. FR-014 ist dort belegt und gehört nicht zum main-API-1-Host. Zusätzliche Supervisoren sind positive Maßnahmen, keine Übertragung einer Releasefreigabe.
* `claude/adoring-hopper-p5zi8e` enthält 91 gegenüber main neue/geänderte Dateien; kein physischer Abnahmenachweis und keine automatischen Rückschlüsse aus dem Branchnamen.
* `module/mountain-keyboard` und `module/thingm-blink`: GPL-Module und Herkunft vorhanden, Stand älter als heutiges main. Ihre ältere Infrastruktur muss vor einem Merge ebenfalls auf heutige Sicherheitskorrekturen geprüft werden.
* `modules/integration` und Backups enthalten ältere/abweichende Installer-, IPC-, Modul- und native Pfade. Eine spätere Zusammenführung darf behobene Grenzen nicht zurücksetzen. Keine der Backupvarianten ist hier als releasefähig freigegeben.
* Tags/Releasepayload unterscheiden sich von main. Ein am heutigen main gefundener Fehler wird nur dann auf einen Tag übertragen, wenn die betreffende Datei/Logik dort ebenfalls nachgewiesen ist. Identische Belegblobs sind pro Risiko dokumentiert.

## Ausgeführte Prüfungen

| Prüfung | Ergebnis | Aussagegrenze |
|---|---|---|
| Git-Blob-Hashprüfung | 2087/2087 passend | Abrufintegrität, keine funktionale Korrektheit |
| `node --test test/api.test.js` in appwrite/community | 1 Integrationsszenario bestanden | Mockbackend; kein produktiver Appwritezugriff |
| `python3 scripts/test-release-preflight.py` | 2 Tests bestanden | Version-/Artefaktprüfung mit synthetischen Dateien |
| `python3 scripts/check-glib-backport.py` | bestanden | Gepinnter Backport in beiden Lockworkspaces korrekt ausgewählt |
| GitHub-CI am main-SHA | CI, Website/shared demo, README progress erfolgreich | Fremdausführung/Statusbeleg; hier nicht erneut ausgeführt |
| Rust-/C#-/Windows-/Hardwaretests | hier nicht ausgeführt | cargo/rustc/dotnet/Windows/Hardware fehlen |

Zwei zusätzliche Regressionen unter `repro/gpu_recovery.rs` sind reine Mocks und sollen die offenen Fehler vor einem Fix zeigen. Sie sind **nicht ausgeführt** und laufen nicht automatisch in der normalen Testsuite. Vollständige Lizenztexte aller Fremdpakete, aktuelle Vulnerabilityscans und Installer-/WASM-Payloadanalyse bleiben ausstehend.

`RISK_REGISTER.md` ist die maßgebliche Befundliste. Dieser Bericht ist ein substanzielles, aber begrenztes Audit, keine lückenlose Zertifizierung oder vollständige Patent-/weltweite Rechtsprüfung.
