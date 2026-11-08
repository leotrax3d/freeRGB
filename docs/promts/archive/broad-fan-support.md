# Prompt: Breite Mainboard-Lüftersteuerung wie FanControl umsetzen

Du arbeitest im Projekt **freeRGB** (Rust-Daemon `freergbd`, C#-Mainboard-Helfer
unter `providers/mainboard`, Tauri-GUI unter `apps/desktop`). Ziel: freeRGB
soll – wie FanControl – die Lüfter an den meisten Mainboards steuern, über
LibreHardwareMonitor (LHM 0.9.6) und PawnIO. Der Projektinhaber hat das
entschieden und nimmt ein geringeres Sicherheitsniveau als bisher bewusst in
Kauf (`docs/DECISIONS.md` #14).

## Arbeitsumgebung (zwingend)

Der Ordner `<repo>` wird von anderen Sitzungen
gleichzeitig benutzt. Arbeite **nie** direkt darin und wechsle dort keinen
Branch. Lege einen eigenen Worktree an:
`git worktree add ../FreeRGB-fans feature/broad-fan-support` (Branch existiert
auf `origin`). Bauen, testen, committen nur dort, mit eigenem `target`-Ordner.
Prüfe vor jedem Commit `git branch --show-current`. Nach jedem fertigen
Arbeitspaket committen und `feature/broad-fan-support` pushen.

## Zuerst lesen

`docs/COOLING_BROAD_SUPPORT_PLAN.md` (der Plan, dein Auftrag),
`docs/DECISIONS.md`, `docs/COOLING_RUNTIME.md`, `docs/MAINBOARD_NATIVE_CONTROL.md`,
`docs/MAINBOARD_CONTROL_RECOVERY.md`, `docs/COOLING_SAFETY.md`,
`docs/FANCONTROL_PARITY.md`, `docs/TESTING.md`, dann den Code:
`providers/mainboard/NativeControl.cs`, `NativeNctEvidence.cs`,
`JournaledNativeControl.cs`, `NativeControlJournal.cs`, `NativeControlWorker.cs`,
`NativeControlSession.cs`, `providers/mainboard/tests/*`,
`crates/freergb-daemon/src/{cooling_host,cooling_runtime,cooling_sessions,native_session,native_bridge}.rs`,
`crates/freergb-control`, `scripts/test-mainboard-*.ps1`.

## Auftrag

Setze die Arbeitspakete aus `COOLING_BROAD_SUPPORT_PLAN.md` **in der dort
festgelegten Reihenfolge** um, jedes vollständig (Code, Tests, Doku) bevor das
nächste beginnt:

1. **LHM-Version zentralisieren** (v0.9.6 ist aktuell die neueste stabile
   Version – kein Upgrade). Die Version steht an einer Stelle; alle Prüfungen
   und Hilfetexte lesen sie von dort.
2. **Gestufte Freigabe:** `NativeNctEvidence` wird optional. Kanäle mit
   geprüftem Register-Rücklesen bekommen die Stufe `verified` (Verhalten
   unverändert). Jeder andere LHM-SuperIO-`Control` mit Software-Modus wird als
   `lhm_supported` zugelassen. Die Stufe wird im Journal, im Protokoll zum
   Daemon, in der API (`cooling.channels.list` u. a.), in der Diagnose und in
   beiden GUIs sichtbar. Keine stille Übernahme eines Kanals, der schon im
   Software-Modus ist (bleibt verboten).
3. **Mehrere Kanäle gleichzeitig:** Journal, Recovery, Worker, Session-Host und
   Rust-Runtime von einem auf alle reservierten Kanäle erweitern
   (pro Kanal eigener Recovery-Eintrag, Freigabe in beliebiger Reihenfolge,
   Crash-Recovery für alle).
4. **Steuerung im Hintergrund:** gespeicherte Kurven pro Kanal laufen ab
   Systemstart im geschützten Dienst, ohne offene GUI.
5. **Geführter Kanaltest und Zuordnung** in API und beiden GUIs (Plan:
   „Safeguards“ 1–2): RPM-Antwort bei 100/70/50 %, nie unter 40 %, Pumpen nur
   100 %, Laufzeit-Plausibilität mit Rückgabe an die Firmware.
6. **Volle Freischaltung für ungetestete Hardware** (Plan: „Unrestricted
   unlock“): globaler Schalter in den Einstellungen (Standard aus), danach pro
   Kanal freischaltbar mit Risikohinweis und Eingabe des Kanalnamens als
   Bestätigung. Freigeschaltete Kanäle (Stufe `unrestricted`) brauchen keinen
   bestandenen Kanaltest, dürfen bis 0 % (Lüfterstopp) gesteuert werden,
   Plausibilitätsabweichungen sind nur Warnungen, und auch Chips ohne
   Rücklesen oder bekanntes Registerschema sind zulässig, solange LHM einen
   `Control` mit Software-Modus liefert. Pumpen brauchen eine zweite, getrennte
   Bestätigung. Die Freischaltung ist an Board, Chip und Kanal gebunden und
   verfällt bei Änderung; sie ist jederzeit widerrufbar.

Danach – wenn Zeit bleibt – die Pakete 6–8 des Plans (Diagnosebericht, Live-Steuerung in der
modernen GUI, fehlende Reglerfunktionen).

## Sicherheitsregeln, die trotz der Entscheidung bleiben

* Ohne Freischaltung: Pumpen und nicht zugeordnete Kanäle werden nie
  gedrosselt, Lüfter nicht unter 40 %, bis ein Kanaltest anderes belegt. Mit
  Freischaltung (Punkt 6) entfallen diese Grenzen für den betreffenden Kanal.
* **Auch mit Freischaltung immer aktiv:** Rückgabe an den erfassten
  Firmware-Modus bei Stopp, Absturz von Dienst/Daemon, Telemetrieverlust und
  Watchdog-Ablauf; Übersteuerung auf 100 % bei kritischer Temperatur; keine
  Übernahme eines Kanals, der schon im Software-Modus ist.
* Jede Rückgabe stellt den vorher erfassten Firmware-Modus wieder her;
  Journal/Watchdog/Recovery gelten für jede Stufe.
* Bestehende Tests dürfen nicht geschwächt werden; das bisherige
  `verified`-Verhalten bleibt byte-genau gleich.
* **Keine Hardwarezugriffe** durch dich: keine Probes, kein Start des
  geschützten Dienstes, keine Treiberinstallation, keine UAC-Aktionen.
  Alles wird mit synthetischen Callbacks/Fixtures getestet (wie die bestehenden
  `providers/mainboard/tests` und `scripts/test-mainboard-*.ps1`).
  Hardwaretests werden später mit dem Nutzer gemacht; schreibe dafür am Ende
  eine Schritt-für-Schritt-Abnahmeliste in den Plan.
* LHM-Interna (private Felder/Methoden) nur dort nutzen, wo es heute schon
  geschieht, mit Versionsprüfung und Test.

## Qualität

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, die vorhandenen PowerShell-Testskripte für den
Mainboard-Helfer (ohne Hardware) und `npm test`/`npx tsc -b` in `apps/desktop`
müssen grün sein. Repo-Sprache Englisch (Code, Kommentare, Doku, Commits).
`docs/STATUS.md` und die betroffenen Dokumente aktualisieren; nichts als
„auf Hardware bestätigt“ markieren.

## Abschluss

Wenn dein Limit naht oder alles erledigt ist: committen, pushen und kurz
zusammenfassen – erledigte Pakete, Tests, offene Punkte, Abnahmeliste für den
Hardwaretest.
