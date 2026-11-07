# Herkunft und Drittcode

Die Commitmetadaten machen die Einführung der GPL-Module nachvollziehbar: `565a110f6d9ed33b131afdfa864435ed937076ab` beschreibt den Razer-Port samt eigener Modul-API, Signaturkette und GPL-Dateien. Die heutige Razer-Provenance listet Ursprungsdateien, Änderungen und Buildbestandteile ausführlich auf. Das ist positive Evidenz gegen eine pauschal versteckte Lizenzübernahme.

42 Hauptbranch-Module sind als GPL-2.0-or-later deklariert. 39 haben weniger als 15 Zeilen Provenance. Alle wurden inventarisiert; daraus folgt keine unabhängige Verifikation ihrer kompletten externen Dateihistorie. RazerController.cpp wurde am gepinnten OpenRGB-SHA separat gelesen; andere externe Familienheader sind noch einzeln gegenzuprüfen. Generatoren übernehmen u.a. Geräte-/Layoutdaten; auch solche Daten dürfen nicht pauschal als schutzlos bezeichnet werden.

Die eingebauten Bragi-/ASUS-/Gigabyte-/ASRock-Protokolle sind anders zu behandeln: Sie beanspruchen eigenständige Implementierung anhand funktionaler Protokollinformationen. `docs/REVERSE_ENGINEERING.md` belegt Einsicht in SignalRGB-Plugins und Logs. Die Behauptung keine Codeübernahme ist ein Entwicklernachweis, keine hier durch umfassenden Textvergleich bewiesene Tatsache. Ein Clean-Room-Team mit nachgewiesener Informationstrennung ist nicht dokumentiert; zwei Agenten desselben Kontextes würden diese Trennung ebenfalls nicht automatisch herstellen.

FanControl ist nach selbst gelesenem Original-Lizenztext proprietär. Bei den untersuchten Manifesten wurde kein Bezug auf FanControl-Binärdateien gefunden; LibreHardwareMonitor ist ein eigenes MPL-Projekt und darf nicht mit FanControl gleichgesetzt werden. UI-/Funktionsähnlichkeit allein beweist keine Urheberrechtsverletzung. Verträge, Schutzmaßnahmen, Geschäftsgeheimnisse oder Patente können andere Grenzen setzen; ihre Verletzung ist hier nicht festgestellt.

KI-Coauthorzeilen und KI-Bearbeitung sind kein Freibrief für fremde Codebestandteile und kein lückenloser Rechtekettenbeleg. Für Beiträge Herkunft, Rechte-/Lizenzbefugnis und wesentliche Referenzen dokumentieren. Eine pauschale Behauptung alles KI-generiert ersetzt diese Prüfung nicht.

Figma-Datei und Exportpipeline für Markenassets sind dokumentiert. Urheber/Beitragende, Fonts, zugekaufte Elemente und etwaige KI-Bedingungen sind noch zu bestätigen. Keine Registrierung oder Verwertung von Schutzrechten im Namen des Nutzers vorgenommen.
