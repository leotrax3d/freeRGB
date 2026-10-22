# Integration von main und feature/tauri-gui

Arbeitsbranch: `codex/integrate-tauri-main`. Kein Merge nach `main` ohne Freigabe.

## Entscheidungen

- [x] Beide Branches und bestehende IPC-/Daemon-Architektur untersuchen.
- [x] Tauri wird Standardoberfläche; Classic bleibt optionaler Fallback (Nutzerfreigabe).
- [x] Windows-Autostart startet ausschließlich den Daemon (Nutzerfreigabe).
- [x] Integrationsbranch auf Basis von main erstellen.
- [x] Tauri-Quellen und Designspezifikation übernehmen; aktuelle Hardware-/Safety-Implementierungen aus main erhalten.

## Implementierung

- [x] Launcher: Tauri als Standard, explizites Classic und Betrieb ohne Fenster.
- [x] Tauri-Daemonstart verwendet installierte Providerkonfiguration des Launchers.
- [x] Windows-Installer, Autostart und Paket-Build integrieren.
- [x] Mainboard-Sensoren und Providerzustände im Monitoring anbinden.
- [x] Geschützte Cooling-Sessions mit expliziter Zuordnung, Status und Stop anbinden.
- [x] Kanalzuordnungen und Kalibrierungsfunktionen auf Abdeckung prüfen: Annotationen editierbar, echte Kalibrierung über geschützte Sessions; vorhandene Policy-Simulation bleibt Backend-/Classic-Werkzeug.
- [x] Tastaturreaktionen mit echtem Daemonzustand konfigurieren.
- [x] Classic-Funktionen (Hotkeys, Anwendungsprofile, Updates) auf Migration/Fallback prüfen: bleiben zunächst in Classic, direkter Start aus Tauri. Hotkeys und Anwendungsprofile benötigen laufendes Classic.
- [x] Verbindungsausfälle, veraltete Werte und fehlende API-Funktionen nachvollziehbar darstellen.
- [x] Mockbetrieb auf ausdrücklich gekennzeichnete Browser-Vorschau begrenzen; Hotkey-Designeditor im Desktopbetrieb durch echten Classic-Zugang ersetzt.

## Qualitätssicherung

- [x] Frontend: Tests, Typprüfung und Produktionsbuild (56 Tests erfolgreich).
- [x] Tauri: Rust-Tests und Windows-Build (8 Tests inklusive Fixture; Clippy ohne Warnungen).
- [x] Rust-Backend und Launcher: relevante Tests und Build.
- [x] Hardwarefreie IPC-Prüfung: vorhandene Daemon- und Cooling-API-Smoke-Tests erfolgreich.
- [x] Start und Schließen des realen Desktopfensters prüfen: gestartet, später per Alt+F4 beendet; Daemon danach weiterhin über API erreichbar und mit rund 30 fps aktiv.
- [x] Installerprüfungen und CI für Tauri ergänzen (lokale Installerkompilierung mangels Inno Setup nicht ausführbar).
- [x] Linux-Unterstützung prüfen; Systembibliotheken und CI ergänzt, lokale Ausführung nicht möglich.
- [x] Hardwareabhängige Funktionen und verbleibende Grenzen dokumentieren.
- [x] Änderungen abschließend prüfen: neue Quellen, Launcher-/Installerpfade, Fehlerzustände und Dokumentation abgeglichen; keine Änderungen an main-Hardware-/Safety-Modulen.

## Noch offene Abnahme vor einem Merge / Release

- [ ] Vollständige visuelle Bedienprüfung der aktuellen Oberfläche auf einem zugänglichen Windows-Desktop.
- [ ] Reale Hardwareprüfung von Mainboard-Sensorik, bestätigten Lüfterzuordnungen, Sessionstart/UAC, Kalibrierung und verifiziertem Stop.
- [ ] Reale Tastaturreaktionen und Zusammenspiel mit Classic-Funktionen prüfen.
- [ ] Neue Linux-CI erfolgreich ausführen; Linux-Fensterstart und Hardwareverhalten prüfen.
- [ ] Installer mit Inno Setup kompilieren und den erweiterten CI-Installationslebenszyklus ausführen.
- [ ] Nutzerfreigabe für einen endgültigen Merge nach main (wird hier nicht durchgeführt).

## Aktueller Stand

Die Tauri-GUI besitzt echte IPC für RGB-/Profil-/Makro-/Sensorfunktionen und
die ergänzten Mainboard-, Session-, Kanalzuordnungs- und Tastaturfunktionen.
Der bestehende Daemon bleibt maßgeblich für Hardwarezugriffe und Sicherheitsprüfungen.
Die Backend-Differenzen des GUI-Branches enthalten bereits in main vorhandene
Custom-Sensor-Funktionen; diese werden nicht nochmals darüberkopiert.

## Prüfgrenzen / Beobachtungen

- Reales Tauri-Fenster mit eingebetteten Produktionsassets gestartet. Accessibility-Baum
  zeigt echten Daemonzustand (30 fps, Effektparameter), keine Mockdaten.
- Windows-Computer-Use liefert hier schwarze Screenshots und keine zuverlässig prüfbare Bedienung;
  der erste Startversuch meldete `GetCursorPos failed: Access is denied (0x80070005)`.
  Visuelle Freigabe und vollständige interaktive Fensterprüfung sind damit noch offen.
- Der per Alt+F4 geschlossene Testprozess war bei der anschließenden Kontrolle beendet;
  der eigene virtuelle Testdaemon lief weiter (API-Version 1, ca. 30 fps, 6274 Ticks).
  Anschließend wurde nur dieser Testdaemon sauber per API beendet.
- Inno Setup ist lokal nicht installiert. Installerkompilierung und der ausschließlich
  auf temporären CI-Runnern erlaubte Installationslebenszyklus sind noch nicht ausgeführt.
- Keine echten Lüfter-/Pumpen-/RGB-Hardwarezugriffe, UAC-Freigaben oder Installationen
  für diese Integration ausgelöst. Linux ist lokal nicht vorhanden.

## Lokale Prüfergebnisse

- `npm run build` und `npm test`: Typprüfung, Produktionsassets und 56 Tests erfolgreich.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked`: 8 Tests erfolgreich.
- Tauri-Clippy mit `-D warnings` und beide Rust-Formatprüfungen erfolgreich.
- Gezielte Tests für freergb-client, freergb-control, freergb-daemon und freergb-gui
  inklusive Launcher erfolgreich. Die sechs vorhandenen expliziten C#-Interoptests
  bleiben ohne ihren separaten Fixture-Aufruf übersprungen.
- `cargo build --workspace --release --locked`: erfolgreich.
- `scripts/build-desktop.ps1`: erfolgreicher Windows-Release-Build mit eingebetteten
  Frontendassets und Kopie nach `target/release/freergb-desktop.exe`.
- Release-Smoke mit virtuellem Prozessplugin: Rendering, Farb-Roundtrip, Plugincrash-
  Isolation/Neustart, Cooling-/Sensor-/Keyboard-API und sauberes Beenden erfolgreich.
- Cooling-Session-Smoke: deaktivierte Sessions, Start-/Stopfehler und ungültige
  Parameter über isolierte echte Named Pipe erfolgreich geprüft.
- PowerShell-Syntaxprüfung der drei geänderten/neuen Build-/Installer-Skripte erfolgreich.
- `git diff --check`: erfolgreich.
