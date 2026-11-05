# Risikoregister

Stand 2026-11-05. Keine numerischen Schadenwahrscheinlichkeiten ohne Daten. **HOCH** bedeutet vor Veröffentlichung beheben oder fachlich klären; **UNGEKLÄRT** ist eine Evidenzkategorie und kein Nachweis geringer Risiken. Kein Brand, Personenschaden oder konkreter Rechtsverstoß wurde experimentell bestätigt. Alle Befunde offen.



| ID | Priorität | Befund |
| --- | --- | --- |
| FR-001 | HOCH | Fehlgeschlagene Rückgabe löscht Wiederherstellungsbedarf |
| FR-002 | HOCH | Journalfehler verhindern Hardwareübernahme nicht |
| FR-003 | HOCH | Linux-/GPU-Notregelung überlebt nicht jeden Daemon- oder Treiberfehler |
| FR-004 | HOCH | Feste Drehzahl hat keinen obligatorischen Temperatur-Notpfad |
| FR-005 | HOCH | Setup verspricht universelle Wiederherstellung, die der Code nicht belegt |
| FR-006 | HOCH | Cloud-Downloadzählung und pseudonyme Konten sind nicht ausreichend transparent |
| FR-007 | HOCH | Vollständige Lizenznachweise für das ausgelieferte Produkt fehlen |
| FR-008 | MITTEL | Anwendungsupdates und Effekte teilen den Online-Vertrauensanker |
| FR-009 | MITTEL | Native Module sind ausführbarer Code mit Benutzerrechten |
| FR-010 | MITTEL | 39 von 42 Modul-Herkunftsnachweisen sind knapp |
| FR-011 | MITTEL | Modulworkflow blockiert alle IDs außer razer-chroma |
| FR-012 | MITTEL | Widerruf der Sicherheitsbestätigung stoppt laufende Kontrolle nicht |
| FR-013 | MITTEL | udev-Regel erteilt Schreibrechte für weitere PWM-Attribute |
| FR-014 | HOCH | Neue Coolingplugins zählen kurze Fail-safe-Schreibvorgänge als Erfolg |
| FR-015 | UNGEKLÄRT | GPL-Grenze und frühere Protokollableitungen benötigen Rechteprüfung |
| FR-016 | UNGEKLÄRT | Geschäftsmodell, FOSS-Status und regulatorische Einordnung sind offen |
| FR-017 | UNGEKLÄRT | Marken, Patente und Assetrechte sind nicht abschließend geklärt |
| FR-018 | HOCH | Freigabenachweise sind unvollständig und nicht auf alle Branches übertragbar |


## FR-001 – Fehlgeschlagene Rückgabe löscht Wiederherstellungsbedarf

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-daemon/src/gpu_cooling.rs:241`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L241), [`main:crates/freergb-daemon/src/gpu_cooling.rs:151`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L151), [`main:crates/freergb-daemon/src/hwmon.rs:599`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L599), [`main:crates/freergb-daemon/src/hwmon.rs:1048`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L1048)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** GPU release entfernt den aktiven Eintrag vor dem Backend-Aufruf; auch bei Rückgabefehler wird das verkleinerte Journal geschrieben. GPU tick und beide Start-Recoverys verwerfen ebenfalls fehlgeschlagene Rückgaben. hwmon release/fault zeigen denselben Verlust; hwmon shutdown erhält das Journal bei Fehler dagegen teilweise korrekt.

**Eintrittsbedingungen:** Manueller Kanal aktiv; Auto-/Firmware-Rückgabe schlägt fehl oder Gerät ist beim Neustart vorübergehend nicht erreichbar.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S03](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Ungeklärter manueller Hardwarezustand kann ohne erneuten Wiederherstellungsversuch fortbestehen; Überhitzung ist ein mögliches, nicht nachgewiesenes Folgerisiko.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Journal, Rückgabefunktionen und Fehlermeldungen vorhanden; Windows JournaledNativeControl räumt erst nach erfolgreicher Rückgabe auf.

**Fehlt:** Dauerhafte unresolved-Liste, erneuter Versuch, nachgewiesene Rückgabe als Löschvoraussetzung.

**Maßnahme:** Einträge bis zum belegten Erfolg behalten, neue Übernahmen bei ungelöstem Zustand sperren; Recoveryfehler sichtbar melden.

**Abnahme:** Mock-Fault-Injection: release-Fehler, fehlendes Gerät, Schreibfehler, Neustart und späterer erfolgreicher Retry; Eintrag bleibt bis zum Erfolg. Zwei GPU-Regressionen in repro/gpu_recovery.rs vorbereitet, hier nicht ausgeführt.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-002 – Journalfehler verhindern Hardwareübernahme nicht

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-daemon/src/gpu_cooling.rs:117`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L117), [`main:crates/freergb-daemon/src/gpu_cooling.rs:216`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L216), [`main:crates/freergb-daemon/src/hwmon.rs:1021`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L1021), [`main:crates/freergb-daemon/src/hwmon.rs:574`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L574)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Die Journalfunktionen liefern keinen Fehler an den aufrufenden Übernahmepfad zurück. Nach fehlgeschlagener Speicherung werden take_control beziehungsweise sysfs-Manual-Schreibvorgänge weiter ausgeführt. Atomic rename allein belegt zudem keine crashfeste Persistenz.

**Eintrittsbedingungen:** Unbeschreibbares/fehlendes Zustandsverzeichnis, voller Datenträger oder Rename-/Persistenzfehler vor der Übernahme.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S03](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Hardware kann manuell gestellt sein, obwohl kein gültiger Recoverydatensatz existiert.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Schreiben vor dem nominellen I/O-Aufruf und temporäre Dateien sind vorhanden.

**Fehlt:** Erfolgsabhängige Aufnahme, persistente Reservierung, ausdrückliche Policy für --no-state.

**Maßnahme:** Journal als fehlbares Transaktionsergebnis behandeln und Übernahme ohne haltbaren Nachweis verweigern.

**Abnahme:** Simuliertes ENOSPC/EACCES/fehlender Pfad: kein take_control und kein Wechsel in Manual; anschließend Recoverytest.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-003 – Linux-/GPU-Notregelung überlebt nicht jeden Daemon- oder Treiberfehler

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-daemon/src/hwmon.rs:453`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L453), [`main:crates/freergb-daemon/src/gpu_cooling.rs:283`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L283), [`main:crates/freergb-daemon/src/engine.rs:938`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/engine.rs#L938), [`main:providers/mainboard/NativeSessionTransport.cs:69`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/providers/mainboard/NativeSessionTransport.cs#L69)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** hwmon besitzt einen Thread im Daemon; GPU-Fan-I/O läuft synchron in Engine::tick. SIGKILL/Abbruch führt keine Drop-Rückgabe aus. Ein Journal wirkt erst beim nächsten Start. Das Windows-Lease-Watchdog ist ein vorhandener Schutz, teilt jedoch Prozess-/Treiberabhängigkeiten und ausdrücklich keine unabhängige Driver-Recovery.

**Eintrittsbedingungen:** Abrupter Prozessverlust, blockierter NVML-/sysfs-/Treiberaufruf, Suspend oder OS-Ausfall bei reduzierter Drehzahl.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S03](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Letzter manueller Stellwert kann länger anliegen; Hardwarethermalschutz ist geräteabhängig und hier nicht als sichere Barriere nachgewiesen.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** 1-s-Regelschritt, Fallbackwerte, Drop, Neustartjournal, systemd Restart; Windows geschützter Worker mit Telemetrie und Lease.

**Fehlt:** Für jeden Backendtyp getesteter Ausfallpfad außerhalb der ausgefallenen Komponente und sichere Begrenzung seiner Wirkungszeit.

**Maßnahme:** Backendabhängiges Sicherheitskonzept und unterstützte Betriebsgrenzen festlegen; Crashgarantien bis zum Nachweis korrigieren; riskante Freigaben begrenzen.

**Abnahme:** In Simulator/VM Hung-call, SIGKILL, Suspend/Resume, Workerverlust testen; reale Rückgabe separat kontrolliert abnehmen. Kein Test an realer Hardware durch dieses Audit.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-004 – Feste Drehzahl hat keinen obligatorischen Temperatur-Notpfad

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-daemon/src/gpu_cooling.rs:292`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L292), [`main:crates/freergb-daemon/src/hwmon.rs:937`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L937), [`main:crates/freergb-daemon/src/gpu_cooling.rs:23`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/gpu_cooling.rs#L23)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Bei Manual wird ein konstanter Sollwert innerhalb des Floors geschrieben. GPU-Backendinterface liefert keine Temperatur/RPM-Notüberwachung; hwmon bestimmt die Temperatur nur für Curve. Der 40%-Floor ist kein Nachweis ausreichender Kühlung unter jeder Last.

**Eintrittsbedingungen:** Nutzer wählt eine formal gültige feste Drehzahl, Kühlbedarf steigt oder physische Kanalzuordnung ist falsch.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S03](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Thermisch unzureichende Regelung trotz gültiger Parameter; tatsächliche Schadensmöglichkeit hängt von Hardware, BIOS und eigener Abschaltung ab.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Floor, Wertevalidierung, bei hwmon Stallprüfung für gepaarten Tachometer; Pumpenstandard 100% sofern korrekt als Pump zugeordnet.

**Fehlt:** Temperaturunabhängige Notregelung in jedem Modus, sichere Identifikation und geprüfte Grenzwerte.

**Maßnahme:** Feste Stellwerte nur unter fortlaufendem Notpfad zulassen; Pumpenrolle und Sensorbindung bestätigen.

**Abnahme:** Bei 40%-Manual und steigender kritischer Testtemperatur springt jeder unterstützte Backendtyp unabhängig von einer Kurve auf einen sicheren Zustand.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-005 – Setup verspricht universelle Wiederherstellung, die der Code nicht belegt

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:apps/desktop/src/features/onboarding/safety.ts:36`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/apps/desktop/src/features/onboarding/safety.ts#L36), [`main:providers/mainboard/NativeControlSession.cs:170`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/providers/mainboard/NativeControlSession.cs#L170), [`main:docs/COOLING_SAFETY.md:96`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/COOLING_SAFETY.md#L96), [`main:docs/SECURITY.md:8`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/SECURITY.md#L8)

**Referenzen mit identischem erstem Beleg:** `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.1`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Der Wizard behauptet Firmware-Rückgabe bei Crash und Sensorverlust für jeden Kanal. Die untersuchten Pfade unterscheiden sich: GPU/Linux siehe FR-001–004; der verified Windows-Notfall kann absichtlich 100% halten und eine Reservierung behalten. Alte Sicherheitsdokumente behaupten zugleich keine reale Fansteuerung. Auch das pauschale unsafe-forbid trifft die NVML-FFI-Ausnahme nicht.

**Eintrittsbedingungen:** Nutzer entscheidet über Freigabe/Überwachung auf Grundlage dieser Aussagen.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S02](LEGAL_SOURCES.md), [S23](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Falsche Sicherheitsentscheidung, unzureichende Instruktion und bei Geschäftstätigkeit mögliche Irreführung; Checkbox heilt die Diskrepanz nicht.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Konkrete Warnung vor Schäden/Brand; zwingende Haftung ist im Disclaimer ausdrücklich ausgenommen.

**Fehlt:** Konsistente, versions- und backendbezogene Sicherheitsinformationen.

**Maßnahme:** Absolute Rückgabebehauptung ersetzen; Notfall-100%, Firmware-Rückgabe und ungeklärte Recovery getrennt beschreiben; bei wesentlicher Änderung Textversion erhöhen.

**Abnahme:** Jede Aussage ist einem getesteten Pfad zugeordnet; keine Zusage für Prozess-/Kernelabsturz ohne Nachweis; juristische Prüfung des Gesamttexts.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-006 – Cloud-Downloadzählung und pseudonyme Konten sind nicht ausreichend transparent

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-effect-host/src/ratings.rs:203`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-effect-host/src/ratings.rs#L203), [`main:crates/freergb-effect-host/src/marketplace.rs:613`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-effect-host/src/marketplace.rs#L613), [`main:appwrite/community/src/main.js:85`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/appwrite/community/src/main.js#L85), [`main:docs/WEBSITE_CONTENT.md:25`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/WEBSITE_CONTENT.md#L25), [`main:appwrite/community/src/cleanup.js:3`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/appwrite/community/src/cleanup.js#L3)

**Referenzen mit identischem erstem Beleg:** `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Normale Konfiguration enthält einen Appwrite-Endpunkt. Nach erfolgreichem Effektdownload werden Identität/Downloaddatensätze erzeugt bzw. übertragen; Server speichert user_id bei Ratings/Downloads/Submissions. Cleanup betrifft quota_slots. In untersuchten Web-/GUI-Dateien fand sich keine vollständige Art.-13-Information oder Löschregelung; die Angabe No cloud or telemetry widerspricht diesem Datenfluss. Der Livebetrieb wurde nicht aufgerufen.

**Eintrittsbedingungen:** Standard-Communitykonfiguration und tatsächliche Nutzung des Online-Marketplace; Zuständigkeit und laufender Betrieb noch festzustellen.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S20](LEGAL_SOURCES.md), [S21](LEGAL_SOURCES.md), [S23](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Betroffene Nutzer, Datenschutzaufsicht; gegebenenfalls klagebefugte Verbände

**Mögliche Folgen/Schadensausmaß:** DSGVO-Informations-/Rechtsgrundlagen-/Speicherbegrenzungsrisiko; mögliche Betroffenenansprüche/Aufsicht. Zufallskennung und Hash sind nicht automatisch Anonymisierung.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Identität serverseitig überprüft; private Tabellen; Reports brauchen explizite Bestätigung; lokale Dev-Quelle sendet keine Zählungen.

**Fehlt:** Verantwortlicher, Zwecke/Rechtsgrundlage, Empfänger/AVV, Aufbewahrung/Löschung, Onlinewahl und konsistente Angaben.

**Maßnahme:** Datenfluss offenlegen, Zählung getrennt steuerbar machen, datensparsame Aufbewahrung/Betroffenenabläufe definieren; §25 TDDDG je Speichervorgang prüfen, keine pauschale Einwilligungspflicht behaupten.

**Abnahme:** Vor Onlinebetrieb dokumentierte Art.-13-Information, konfigurierte Löschung einschließlich Konten/Reports und nachvollziehbare Datenflussprüfung; DPA/Drittlandprüfung sofern erforderlich.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-007 – Vollständige Lizenznachweise für das ausgelieferte Produkt fehlen

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:packaging/windows/freergb.iss:77`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/packaging/windows/freergb.iss#L77), [`main:apps/desktop/package.json:16`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/apps/desktop/package.json#L16), [`main:packaging/windows/mainboard-packages.json:8`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/packaging/windows/mainboard-packages.json#L8)

**Referenzen mit identischem erstem Beleg:** `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Installer-Skript bindet Projektlizenzen, GLib-Nachweise und LHM-Notices ein, aber keinen nachweisbar vollständigen Rust-/Frontend-/Font-/Systembibliotheks-Nachweissatz. Desktop-Lock deklariert u.a. Inter als OFL-1.1, React als MIT und Lucide als ISC. Ob konkrete Lizenztexte im fertigen EXE-Payload vorhanden sind, wurde ohne Binärextraktion nicht geprüft. Daher Nachweislücke, keine pauschale Feststellung einer bereits erfolgten Verletzung.

**Eintrittsbedingungen:** Weitergabe eines Binärprodukts mit einschlägigen Fremdkomponenten.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S14](LEGAL_SOURCES.md), [S18](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Rechteinhaber der tatsächlich enthaltenen Crates, Fonts, Icons und Laufzeitbibliotheken

**Mögliche Folgen/Schadensausmaß:** Bei tatsächlich fehlenden Pflichttexten Unterlassungs-/Nachlizenzierungs-/Kostenrisiken gegenüber jeweiligen Rechteinhabern.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Projektlizenztexte, LHM/PawnIO-Notices und GLib-Backportnachweis vorhanden.

**Fehlt:** Vollständiger artefaktbezogener Lizenzgraph und installierter Notice-/Fontlizenzsatz, Prüfung nativer LGPL-Komponenten auf Linux.

**Maßnahme:** Aus aufgelöstem Zielplattformgraph vollständige Notices erzeugen und Installer/Pakete extrahieren und abgleichen; SBOM mit derzeitigen NOASSERTION-Einträgen vervollständigen.

**Abnahme:** Jede tatsächlich gelieferte Komponente hat geprüften Lizenztext, erforderliche Attribution/Quellcodezugang und passenden Artefaktnachweis.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-008 – Anwendungsupdates und Effekte teilen den Online-Vertrauensanker

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-update/src/lib.rs:119`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-update/src/lib.rs#L119), [`main:crates/freergb-update/src/lib.rs:310`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-update/src/lib.rs#L310), [`main:crates/freergb-effect-host/src/marketplace.rs:2`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-effect-host/src/marketplace.rs#L2)

**Referenzen mit identischem erstem Beleg:** `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Der Anwendungsinstaller wird gegen einen GitHub-Assetdigest geprüft und später erhöht gestartet. Eine zusätzliche Entwickler-Signaturprüfung ist im untersuchten Pfad nicht implementiert. Effekte nutzen ebenfalls TLS und Kataloghashes; Hardwaremodule haben dagegen eine eigene Ed25519-Kette. Ein Hash beweist Byteintegrität gegenüber Metadaten, nicht Unabhängigkeit von kompromittiertem Repositorykonto.

**Eintrittsbedingungen:** Repository-/Releasekonto oder dessen Auslieferung wird kompromittiert. Kein solcher Vorfall nachgewiesen.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S08](LEGAL_SOURCES.md), [S01](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Manipulierte Software könnte als legitim erscheinen; bei Installer hohe lokale Rechte.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** HTTPS, fester Repositorypfad, Größen-/Hashprüfung, neu exklusiv angelegte Stagingdatei, Verifyhandle vor UAC; WASM-Limits.

**Fehlt:** Zusätzliche Publisher-/Offlinesignatur und dokumentiertes Revocation-/Incidentverfahren.

**Maßnahme:** Signierte Anwendungsmanifeste/Authenticode mit überprüftem Publisher sowie Schlüsselwechsel und Wiederherstellung planen.

**Abnahme:** Manipuliertes Paket und manipulierte Onlinehash-Metadaten ohne gültigen unabhängigen Signaturschlüssel werden verworfen.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-009 – Native Module sind ausführbarer Code mit Benutzerrechten

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-modules/src/host.rs:54`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-modules/src/host.rs#L54), [`main:crates/freergb-modules/src/host.rs:57`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-modules/src/host.rs#L57), [`main:docs/SECURITY.md:25`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/SECURITY.md#L25)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/sensors-gpu`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Prozessisolation, Brokerregeln und Signaturen beschränken den IPC-Verkehr, sind aber keine OS-Sandbox des nativen Programms. Es kann grundsätzlich Benutzerdateien/Netzwerk nutzen und eigene Kindprozesse starten. Die Risiken gelten auch für die neuen Coolingplugins; diese als untrusted zu bezeichnen schafft keine technische Eindämmung.

**Eintrittsbedingungen:** Vertrauenswürdiger Signaturschlüssel gibt bösartigen/fehlerhaften Code frei oder lokaler Programmkontext ist kompromittiert.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S08](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Nutzerdateien/Onlinezugang und Hardware im Rahmen verfügbarer Rechte betroffen.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Ed25519-Katalog, Hashes, Prozesszeitlimits und begrenzte Brokeraufrufe; WASM-Effekte sind separat ohne Hostimports beschränkt.

**Fehlt:** Explizite Publishervertrauensentscheidung oder echte OS-Sandbox und Kindprozessbegrenzung.

**Maßnahme:** Vertrauensmodell klar dokumentieren; falls bösartige native Erweiterungen abgefangen werden sollen, AppContainer/seccomp bzw. vergleichbare passende Isolation entwerfen.

**Abnahme:** Sicherheitsbeschreibung unterscheidet Crashisolation von Sandbox; adversariales Testprogramm kann definierte verbotene Ressourcen nicht erreichen, wenn eine Sandbox zugesagt wird.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-010 – 39 von 42 Modul-Herkunftsnachweisen sind knapp

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:modules/corsair-hydro-platinum/PROVENANCE.md:4`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/modules/corsair-hydro-platinum/PROVENANCE.md#L4), [`main:modules/razer-chroma/PROVENANCE.md:21`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/modules/razer-chroma/PROVENANCE.md#L21)

**Referenzen mit identischem erstem Beleg:** `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** 42 main-Module tragen GPL-Lizenzmetadaten und COPYING. 39 PROVENANCE-Dateien haben unter 15 Zeilen, oft nur Familie und Commit. Das ist deutlich schwächer als der Razer-Nachweis mit Datei-/Autor-/Änderungszuordnung. Der Razer-Upstreamheader wurde unabhängig gegengeprüft; nicht jeder verwendete Header der übrigen Familien.

**Eintrittsbedingungen:** Veröffentlichung/Weitergabe von weiteren Modulbinärdateien oder Streit über Umfang/Lizenz einer Übernahme.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S14](LEGAL_SOURCES.md), [S15](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Herkunfts- und Lizenzbeweis kann unzureichend sein; ein kurzer Nachweis allein beweist noch keine Rechtsverletzung.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** GPL-Kennzeichnung, COPYING, gepinnter Ursprungscommit, Generatoren, eigener Quellcode verfügbar.

**Fehlt:** Datei-/Autor-/Lizenzzuordnung und Änderungen je Familie sowie tatsächliche Source-Artefakte.

**Maßnahme:** Für jedes Modul Fremdquelle, Header, Autoren, Übernahmen und Änderungen dokumentieren; Originalhinweise in angemessener Form erhalten.

**Abnahme:** Jede Ursprungsdatei ist am konkreten Commit geprüft und jedem ausgelieferten Modul eindeutig zugeordnet.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-011 – Modulworkflow blockiert alle IDs außer razer-chroma

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:.github/workflows/modules.yml:42`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/.github/workflows/modules.yml#L42), [`main:.github/workflows/modules.yml:266`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/.github/workflows/modules.yml#L266)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Build und Sourcejob haben weiterhin einen exklusiven Razer-ID-Guard, obwohl nachfolgende Generatorzweige viele andere Familien kennen. Deshalb ist der gezeigte Releaseweg kein Nachweis einer vollständigen Auslieferung aller 42 Module. In den vier gefundenen Releases liegt kein modules-Katalog.

**Eintrittsbedingungen:** Geplante Veröffentlichung weiterer Module oder manuelle Umgehung dieses Workflows.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S14](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Fehlende Verfügbarkeit oder außerhalb des geprüften Prozesses gebaute Binär-/Quellcodepaare; keine bereits belegte GPL-Verletzung.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Bei Razer: Paket, Sourcearchive, eigenständiger Sourcebuild, signierter Katalog; Publikation manuell/draft.

**Fehlt:** Generischer oder bewusst begrenzter und entsprechend deklarierter Releaseumfang.

**Maßnahme:** Erlaubte Module überprüfbar aus einer Freigabeliste bauen und für jedes die Sourceprüfung ausführen; Dokumentationsversprechen daran ausrichten.

**Abnahme:** Für jede freigegebene ID sind Binärpaket und passendes Quellarchiv gemeinsam erzeugt, geprüft und für Empfänger erreichbar.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-012 – Widerruf der Sicherheitsbestätigung stoppt laufende Kontrolle nicht

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:crates/freergb-daemon/src/api.rs:506`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/api.rs#L506), [`main:crates/freergb-daemon/src/onboarding.rs:82`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/onboarding.rs#L82), [`main:crates/freergb-daemon/src/hwmon.rs:438`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/hwmon.rs#L438), [`main:crates/freergb-daemon/src/main.rs:240`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-daemon/src/main.rs#L240)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/sensors-gpu`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** safety.revoke ändert nur Bestätigungsstatus. Es ruft keinen Stop-/Releasepfad auf. Der Linuxstart lädt gespeicherte Assignments direkt über assign, ohne die API-Sicherheitsabfrage. Das ist für die Erklärung Fan control locks again unvollständig; automatische Wiederaufnahme kann trotz widerrufener Bestätigung stattfinden.

**Eintrittsbedingungen:** Nutzer widerruft bei aktivem Kanal bzw. gespeicherten Linux-Assignments und startet später erneut.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S02](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Weiterlaufende/wiederaufgenommene Kontrolle trotz erwarteter Sperre; Erwartungs- und Bedienrisiko.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Neue API-Assignments prüfen require_safety; Versionierung und Zeitstempel vorhanden.

**Fehlt:** Backendübergreifender Stop bei Widerruf und erneute Zulassung vor automatischer Wiederaufnahme.

**Maßnahme:** Widerruf als kontrollierten Stop/Drain gestalten; Startpfad derselben Freigabelogik unterwerfen; Fehlerzustände nicht als erfolgreichen Stop melden.

**Abnahme:** Revoke während aktiver Mock-Kanäle stoppt sie mit Rückgabenachweis; Neustart ohne aktuelle Zustimmung übernimmt keine gespeicherten Kanäle.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-013 – udev-Regel erteilt Schreibrechte für weitere PWM-Attribute

**MITTEL · P1 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:packaging/linux/60-freergb.rules:40`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/packaging/linux/60-freergb.rules#L40)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Der Shellglob pwm[0-9]* erfasst nicht nur pwmN und pwmN_enable, sondern je Treiber auch pwmN_mode, pwmN_freq und automatische Kurvenattribute. Die Regelbeschreibung nennt nur die beiden nötigen Attribute.

**Eintrittsbedingungen:** Gruppe freergb wird installiert und hwmon-Gerät bietet zusätzliche PWM-Dateien.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Größere als beschriebene Konfigurations-/Schadensfläche für ohnehin autorisierte Gruppenmitglieder; kein belegter Fremdzugriff auf nicht autorisierte Konten.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Explizite Gruppe und begrenzte Treibernamensliste.

**Fehlt:** Exakte Attributfreigabe und Prüfung jeder tatsächlich benötigten Eigenschaft.

**Maßnahme:** Nur exakt benötigte pwmN-/pwmN_enable-Dateinamen freigeben und dokumentieren; sonstige Attribute nicht automatisch chmoden.

**Abnahme:** Fixture mit pwm1, pwm1_enable, pwm1_mode, pwm1_freq, pwm1_auto_point1_pwm: nur die ausdrücklich freigegebenen Dateien erhalten Gruppenrechte.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-014 – Neue Coolingplugins zählen kurze Fail-safe-Schreibvorgänge als Erfolg

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`cooling/plugins:crates/freergb-modules/src/cooling_host.rs:466`](https://github.com/leotrax3d/FreeRGB/blob/26588ad52d16fff0cdaeeee131ec877bcee96b05/crates/freergb-modules/src/cooling_host.rs#L466), [`cooling/plugins:crates/freergb-modules/src/cooling_host.rs:232`](https://github.com/leotrax3d/FreeRGB/blob/26588ad52d16fff0cdaeeee131ec877bcee96b05/crates/freergb-modules/src/cooling_host.rs#L232)

**Referenzen mit identischem erstem Beleg:** `cooling/gui-parity`, `cooling/plugins`, `cooling/service-profiles`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Im Fail-safe zählt jeder Ok-Rückgabewert als erfolgreich geschriebener Report. Die erwartete Reportlänge wird nicht mit der zurückgegebenen Bytezahl verglichen. Ein kurzer Transfer kann dadurch als erfolgreicher Recoveryreport erscheinen. Der IPC-Deadline begrenzt zudem keine bereits im selben Hostthread blockierte native write/send_feature-Operation.

**Eintrittsbedingungen:** Nur neue API-3-Coolingpfade in cooling/plugins, cooling/service-profiles und cooling/gui-parity; Transport liefert kurzen erfolgreichen Count oder blockiert im Treiber.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S08](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Überbewerteter Fail-safe-Nachweis; tatsächlicher Hardwarezustand bleibt ungeklärt. Kein realer Fehlertransfer ausgeführt.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Signierte Fail-safe-Policy, Prozessbeendigung vor eigener Rückgabe, Lease-/Freshness-Prüfung und Ergebnisfeld errors vorhanden.

**Fehlt:** Vollständige Transferprüfung, expliziter ungeklärter Recoveryzustand und Treiber-Livenessnachweis.

**Maßnahme:** Counts exakt prüfen, kurze Transfers als Fehler führen; neue Pluginpfade erst nach Mock-Fault-Injection und eigener Hardwarefreigabe zusammenführen.

**Abnahme:** Mocktransport Ok(0), Ok(len-1), Err und blockierter Aufruf; kein kurzer Transfer erhöht written oder erlaubt erfolgreichen Recoverystatus.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-015 – GPL-Grenze und frühere Protokollableitungen benötigen Rechteprüfung

**UNGEKLÄRT · Klärung · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:docs/REVERSE_ENGINEERING.md:19`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/REVERSE_ENGINEERING.md#L19), [`main:crates/freergb-protocols/src/corsair_bragi/mod.rs:4`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-protocols/src/corsair_bragi/mod.rs#L4), [`main:docs/HARDWARE_MODULES.md:45`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/HARDWARE_MODULES.md#L45)

**Referenzen mit identischem erstem Beleg:** `backup/modules-integration-remote`, `backup/modules-integration-wip`, `backup/tauri-gui`, `backup/website-live-demo`, `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `module/mountain-keyboard`, `module/thingm-blink`, `modules/integration`, `refs/tags/prerelease-broad-fan-support-1`, `refs/tags/v0.1.0-beta.1`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** OpenRGB-Module sind ausdrücklich abgeleitete Werke. Ob die konkrete enge Brokerkommunikation getrennte Programme oder ein rechtlich einheitliches Werk bildet, ist keine allein durch Prozessgrenzen entschiedene Frage. In-tree-Protokolle berufen sich auf unabhängig formulierte Funktionsfakten nach Quelltexteinsicht; ein unabhängiges Clean-Room-Verfahren ist damit nicht belegt. Kein unerlaubtes Kopieren von SignalRGB-/FanControl-Code hier nachgewiesen.

**Eintrittsbedingungen:** Weitergabe von Modul/Host-Kombination oder Beanstandung von konkreter Ausdrucksübernahme/Vertragsverstoß.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S11](LEGAL_SOURCES.md), [S12](LEGAL_SOURCES.md), [S13](LEGAL_SOURCES.md), [S14](LEGAL_SOURCES.md), [S16](LEGAL_SOURCES.md), [S17](LEGAL_SOURCES.md), [S30](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Jeweilige OpenRGB-Autoren; bei konkret belegter Übernahme SignalRGB-Rechteinhaber; FanControl-Rechteinhaber nur bei tatsächlich betroffener Rechtsposition

**Mögliche Folgen/Schadensausmaß:** Je nach Tatsachen/Einordnung Lizenzänderungsbedarf, Entfernung einzelner Übernahmen, Unterlassung oder Kosten; keine konkrete Anspruchswahrscheinlichkeit bestimmbar.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Bewusste GPL-Trennung, eigene Protokollbeschreibungen, offengelegte Herkunft, keine FanControl-Abhängigkeit gefunden.

**Fehlt:** Dateigenaue Vergleichsprüfung, Verträge der tatsächlich genutzten Referenzversion und juristische Einordnung der Kommunikation.

**Maßnahme:** Fachanwalt mit konkreten Dateien/Historieständen prüfen lassen; Funktionen/Fakten von schutzfähigen Tabellen/Strukturen trennen. Keine nachträgliche Clean-Room-Behauptung.

**Abnahme:** Begründete, schriftliche Lizenzgrenzen-/Herkunftsbewertung und Beseitigung konkret festgestellter Übernahmen; jeder Sourceempfänger erhält die nötigen Rechte.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-016 – Geschäftsmodell, FOSS-Status und regulatorische Einordnung sind offen

**UNGEKLÄRT · Klärung · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:README.md:179`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/README.md#L179), [`main:crates/freergb-effect-host/src/ratings.rs:204`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/crates/freergb-effect-host/src/ratings.rs#L204)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/sensors-gpu`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Repo ist beim Audit privat; Empfängerkreis und tatsächlich offengelegter Quellcode, Monetarisierung, Sponsoring/Gegenleistungen, Betreiberrechtsträger, Zielmärkte und tatsächlicher Onlinebetrieb sind nicht aus dem Code feststellbar. Das entscheidet über CRA/FOSS-Ausnahmen, Produkthaftungsreform und Dienstepflichten. Privates Repository allein beweist weder Verstoß noch anwendbare FOSS-Ausnahme.

**Eintrittsbedingungen:** Öffentliche Abgabe, kommerzielle Angebote oder Verarbeitung fremder Daten.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S05](LEGAL_SOURCES.md), [S06](LEGAL_SOURCES.md), [S07](LEGAL_SOURCES.md), [S08](LEGAL_SOURCES.md), [S09](LEGAL_SOURCES.md), [S10](LEGAL_SOURCES.md), [S22](LEGAL_SOURCES.md), [S26](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Je nach Sachverhalt zusätzliche Melde-, Sicherheits-, Informations-, Haftungs- oder Verbraucherpflichten.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Quelllizenz und Sicherheitsdokumentation vorhanden.

**Fehlt:** Dokumentierter Betreiber-/Vertriebs-/Finanzierungssachverhalt und darauf bezogene Rechtsbewertung.

**Maßnahme:** Anwendbarkeit vor Freigabe schriftlich entscheiden, CRA-Art.-14-Datum nicht erst auf 2027 verschieben; neue deutsche Produkthaftungsfassung im Original nachprüfen.

**Abnahme:** LAWYER_REVIEW.md Sachverhaltsfragen beantwortet; Anwendbarkeitsmatrix mit Stichtagen freigegeben.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-017 – Marken, Patente und Assetrechte sind nicht abschließend geklärt

**UNGEKLÄRT · Klärung · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:assets/ASSETS.md:7`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/assets/ASSETS.md#L7), [`main:README.md:2`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/README.md#L2)

**Referenzen mit identischem erstem Beleg:** `backup/modules-integration-remote`, `backup/modules-integration-wip`, `backup/website-live-demo`, `claude/adoring-hopper-p5zi8e`, `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`, `module/mountain-keyboard`, `module/thingm-blink`, `modules/integration`, `refs/tags/prerelease-broad-fan-support-1`, `refs/tags/v0.1.0-beta.1`, `refs/tags/v0.1.0-beta.2`, `refs/tags/v0.1.0-beta.3`, `refs/tags/v0.1.0-beta.4`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Figma-Export ist ein Herkunftshinweis, kein Beleg sämtlicher Rechte an Logo, Schrift und zugelieferten Elementen. Es erfolgte keine systematische DPMA/EUIPO/WIPO-/Patentfamilienrecherche. Kein konkretes entgegenstehendes Schutzrecht und keine tatsächliche Verletzung festgestellt.

**Eintrittsbedingungen:** Veröffentlichung/Verwendung des Namens, Designs oder einer fremd geschützten technischen Lehre.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S28](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Nur konkret festzustellende Urheber, Kennzeichen- oder Patentinhaber

**Mögliche Folgen/Schadensausmaß:** Unbekannt; keine erfundenen Markeninhaber oder Patentansprüche.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Eigene SVGs und dokumentierte Exportquelle; Fonts in Paketmetadaten auffindbar.

**Fehlt:** Rechtekette der Assets, Schriftbedingungen, Kennzeichen-/gegebenenfalls Patentclearance.

**Maßnahme:** Assetherkunft bestätigen und bei relevantem Marktauftritt Kennzeichenrecherche; Patentprüfung anhand konkret implementierter Funktionen statt pauschalem Internettreffer.

**Abnahme:** Registrierte/benutzte Kennzeichen und relevante Patentansprüche fachlich abgeglichen; Autoren-/Lizenzzuordnung dokumentiert.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen


## FR-018 – Freigabenachweise sind unvollständig und nicht auf alle Branches übertragbar

**HOCH · P0 · Statische Code-/Konfigurationsprüfung; keine Schadensbestätigung**

**Codebelege:** [`main:.github/workflows/package.yml:49`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/.github/workflows/package.yml#L49), [`main:docs/BETA_READINESS_AUDIT.md:156`](https://github.com/leotrax3d/FreeRGB/blob/fb1a0af8f762104e017f0b56341415ee609e5d40/docs/BETA_READINESS_AUDIT.md#L156)

**Referenzen mit identischem erstem Beleg:** `cooling/controller-features`, `cooling/gui-parity`, `cooling/plugins`, `cooling/sensors-gpu`, `cooling/service-profiles`, `main`. Dies ist eine Blobgleichheit, keine Aussage über identische Gesamtlaufzeit; abweichende Varianten siehe Branchmatrix.

**Befund:** Die Umgebung hat kein cargo/rustc, dotnet oder Windows. Installer-/WASM-/Bildbinärdateien sind nicht extrahiert/verifiziert. Vollständige Fremdpaket-Lizenztexte und aktuelle Advisoryscans fehlen. 2087 eindeutige UTF-8-Blobs wurden hashgeprüft; das ist keine manuelle Vollprüfung jeder Zeile. Historienmetadaten wurden vollständig erfasst, ausgewählte Änderungen gelesen, nicht alle 530 Diffs. Das frühere Audit ersetzt diese offenen Nachweise nicht.

**Eintrittsbedingungen:** Freigabe ohne Nachholen der fehlenden Tests/Artefakt- und Lizenzprüfung.

**Rechtsraum/Grundlage:** Deutschland/EU; ausländische Anspruchsgrundlagen separat zu prüfen; [S01](LEGAL_SOURCES.md), [S08](LEGAL_SOURCES.md), [S29](LEGAL_SOURCES.md)

**Mögliche Anspruchsteller:** Geschädigte Nutzer/Eigentümer; bei versicherten Schäden gegebenenfalls übergegangene Ansprüche von Versicherern

**Mögliche Folgen/Schadensausmaß:** Unberechtigte Sicherheits-/Compliancefreigabe bei verbleibenden hohen Befunden und ungeklärten Komponenten.

**Wahrscheinlichkeit:** Nicht quantifiziert; an konkrete Fehlerbedingungen gebunden

**Unsicherheit:** Codebefund sicher; physische Wirkung und konkrete Rechtsfolge nicht experimentell bestätigt

**Vorhanden:** Drei am main-SHA erfolgreiche CI-Workflowstatus, vorhandene Simulationssuite; hier 1 Node-Integrationsszenario, 2 Python-Tests und GLib-Integritätsprüfung bestanden.

**Fehlt:** Neue Fehlerregressionen, vollständige Zielplattformgraphen/Lizenztexte, Binärinhaltsprüfung und kontrollierte Hardwareabnahme.

**Maßnahme:** Als begrenztes Audit dokumentieren; keine Zertifizierung. Fehler beheben und an exakt zu veröffentlichendem Commit Prüfungen/Artefakte zusammenführen.

**Abnahme:** Alle P0-Abnahmekriterien erfüllt, vollständiger Testbericht je freigegebenem Branch/Release; SBOM-Lücken geschlossen oder klar fachlich akzeptiert.

**Freigabe:** Technische Korrektur und Nachweis; rechtliche Bewertung bei Veröffentlichung mit Fachanwalt abstimmen
