# Priorisierte Maßnahmen und Abnahme

Keine Produktkorrekturen Teil dieses Auditcommits. Maßnahmen sind technisch prüfbar formuliert; eine Warnung allein schließt keinen Hardwarebefund.

| Reihenfolge | Befunde | Maßnahme | Abnahme |
|---|---|---|---|
| P0.1 | FR-001/002 | Dauerhafte Recoverytransaktion für GPU/hwmon; fehlende Journalpersistenz blockiert I/O | Mock-Fehlersequenzen und Neustart behalten unresolved-Einträge bis belegtem Erfolg |
| P0.2 | FR-003/004 | Backendunabhängige Notregelung/ausdrückliche Betriebsgrenzen und Temperaturinterlock in Manual | Hang/Kill/Suspend/Missing sensor führen innerhalb geprüfter Frist zum sicheren Zustand; keine reale Garantie allein aus Modell |
| P0.3 | FR-005 | Sicherheitszusagen/Wizard/Dokumentation versionsbezogen berichtigen | Jede behauptete Barriere ist implementiert und mit passendem Nachweis belegt; Textversion und Abnahme |
| P0.4 | FR-006 | Onlinezwecke/Rechtsgrundlage/Information/Löschung und Offline-/Zählwahl umsetzen | Dokumentierte tatsächliche Datenflüsse, Retention und Verantwortlichkeit vor Onlinebetrieb |
| P0.5 | FR-007/015/016 | Artefaktbezogene Lizenzprüfung, Sourcezugang, FOSS-/Geschäftsrolle und Haftungstext klären | Juristisch geprüfte Rechte-/Anwendbarkeitsmatrix; kein offenes hohes Lizenzrisiko |
| P0.6, vor Merge | FR-014 | Short-write-/Hung-driver-Grenze neuer API3-Plugins | 0/short/error Transfers ohne fälschlichen Fail-safe-Erfolg; getrennte Recoverybelege |
| P0.7 | FR-018 | Alle fehlenden Zielplattform-, Binär-, Lizenz- und Sicherheitsprüfungen nachholen | Exakter Release-SHA, Artefakthashes, Test-/Notice-/SBOM-Abgleich und ausdrückliche begrenzte Hardwarefreigabe |
| P1 | FR-008/009 | Unabhängige Updatesignatur und präzises natives Vertrauensmodell | Gefälschte Online-Metadaten reichen nicht für erhöht ausgeführten Code; Sandboxbehauptungen nur bei OS-Nachweis |
| P1 | FR-010/011 | Herkunft je Modul vertiefen und kontrollierten Build-/Sourceweg erweitern | Jede freigegebene ID hat bestätigte Ursprungsdateien und getestetes Source-/Binärpaar |
| P1 | FR-012/013 | Consent-Widerruf und exakte Linux-Dateirechte | Aktive Kontrolle beim Widerruf gestoppt, keine automatische ungeprüfte Wiederaufnahme, kein chmod weiterer Attribute |
| Klärung | FR-017 | Name/Assets/gegebenenfalls Patente | Rechtekette und zielmarktbezogene Freigabe, keine behauptete allgemeine Patentfreiheit |

Nach jeder Korrektur nur betroffene und notwendige übergreifende Regressionen ausführen. Anschließend erneute Gesamtschau am tatsächlichen Release-SHA. Frühere erfolgreiche CI-Läufe, andere Branches oder eine Bezeichnung Beta sind keine Ersatzabnahme. Keine automatischen realen Hardwaretests, keine produktiven Releases oder Lizenzänderungen ohne gesonderte ausdrückliche Freigabe.
