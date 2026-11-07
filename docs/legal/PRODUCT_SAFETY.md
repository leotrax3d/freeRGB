# Produktsicherheit und Fehlerszenarien

**Keine uneingeschränkte Hardwarefreigabe.** FR-001–005 und FR-014 sind vorrangig. Registerbelege und Abnahmefälle stehen im Risikoregister; keine tatsächliche Hardwarebeschädigung behauptet.

| Szenario | Implementierter Schutz | Noch notwendiger Nachweis |
|---|---|---|
| Fehlende/alte Temperatur in Kurve | Curve-/Telemetrieguards verlangen hohe Drehzahl | Backendweit und unter realen Fehlern; Manual siehe FR-004 |
| RPM 0 | hwmon-Stallticks; Windows-Telemetrieguard | Richtiger physischer Tachometer, Pumpen und Gerätebesonderheiten |
| Falsche Zuordnung | Explizite Assignments, Windows-Channel-Test/Bindings | Fanindex ist kein universeller Beweis physischer Verkabelung |
| Daemon normal stoppen | Drop/release/Shutdownpfade | Erfolgreiche reale Rückgabe, Fehlerzustände FR-001 |
| Daemon SIGKILL/Crash | Recoveryjournal/Neustart; Windows eigener Worker | Linux-/GPU-Ausfallzeit, dauerhaft fehlender Neustart, FR-002/003 |
| Treiber-/Lockhang | Windows Workerlease; neue Service-Supervisoren | Recovery darf nicht auf derselben ausgefallenen Ressource blockieren |
| Ungültige Kurve/NaN | Eingabevalidierung und Floors | Gesamte Pfadwirkung; 40% ist kein universell sicherer thermischer Wert |
| Firmware übernimmt Kanal | hwmon einmalige Reassertion, dann Fault | Nicht gegen Sicherheitsentscheidung der Firmware arbeiten; Abnahme pro Treiber |
| Schreib-/Restorefehler | Fehlerlogging und Fallbackversuche | Zustand dauerhaft unresolved halten, keine falschen Erfolgsbelege |
| Standby/Resume/Boot | vorhandene Sessions/Journale und Profile | Gezielte Simulation und reale kontrollierte Betriebssystemabnahme |
| Fehlerhafter RGB-Report | Adapter-/Report-/Bereichschecks, bekannte Geräte | Strombudget/ARGB-Konfiguration und gefährliche Befehle je Hardware |
| Modulfehler/-Manipulation | Signatur, Hashes, Prozessgrenzen; bei API3 signierte Fail-safe-Reports | FR-009/014; Signatur ist kein Sicherheitsbeweis des Codes |
| Update/Deinstallation | Maintenance-/Stoplogik vorhanden | Aktive Kanäle, Abbruch/Rollback, fehlgeschlagene Rückgabe und Binärprüfung |

Es gibt keinen belegten Pfad, der allein wegen einer RGB-Farbwahl sicher einen Kurzschluss oder Brand erzeugt. Hohe LED-Anzahl/Helligkeit kann abhängig von Hardware/Stromversorgung Last erhöhen; konkrete Schutz- und Stromgrenzen sind ohne Gerätespezifikationen nicht nachgewiesen. Firmware-Flash-/Spannungs- oder Pinbelegungsänderungen wurden nicht getestet. Gefährdungsmodell: Fehler → ausbleibende Kühlung → erhöhte Temperatur → abhängig von Hardware Abschaltung, Datenverlust oder Sachschaden; Brand/Personenschaden erfordern zusätzliche konkrete Ursachen- und Zurechnungsprüfung.

Eine GUI-Warnung ersetzt keinen unabhängigen Notpfad. Vor Gerätefreigabe: genaue Modell-/Firmwarekennung, Ausgangszustand, elektrische/physische Zuordnung, getrennter Temperatursensor, Obergrenzen, Abbruchplan und dokumentierte Rückgabe. Die gefährlichen Abnahmen bleiben ausdrücklich zustimmungspflichtig; dieses Audit hat ausschließlich Dateien/Mocks benutzt.
