# Gemeinsame Desktop-GUI und Browser-Demo

## Architekturentscheidung

Die moderne Desktop-GUI liegt unter `apps/desktop`: React 19, TypeScript,
Tailwind 4 und Tauri 2. Die Tauri-Schicht übersetzt UI-Aufrufe in lokale
Daemon-RPCs und liefert State-Slices. Der Rust-Daemon läuft unabhängig von
der Oberfläche. Die weiterhin verfügbare Classic-GUI unter
`crates/freergb-gui` verwendet egui, native Client-Verbindungen und optionale
Windows-Integrationen. Sie wird nicht für die Browser-Demo kompiliert.

Die Website ist eine eigenständige React-18-/Vite-Anwendung mit Tailwind 3.
Website und Desktop verwenden ähnliche Gestaltung, aber unterschiedliche
CSS-Systeme und React-Versionen. Es gab bisher keinen Website-Deployment-Workflow.

Verglichene Ansätze:

| Ansatz | Wiederverwendung | Aufwand / Einschränkung |
| --- | --- | --- |
| Vorhandenes React-Frontend separat bauen | Vollständige moderne UI einschließlich Styles und Navigation | Bestehenden Mock-Transport absichern; keine Migration |
| UI direkt in Website importieren | Gleiche Komponenten | React-/Tailwind-Versionen vereinheitlichen und globale Styles isolieren |
| Classic per WebAssembly portieren | Ältere UI | Native IPC, Threads und Plattformintegrationen erfordern zusätzliche Adapter; bildet die moderne UI nicht ab |
| Separate HTML-Nachbildung | Keine dauerhafte Synchronität | Zweite zu wartende Oberfläche; verworfen |

Gewählt: zwei Builds desselben `apps/desktop/src/main.tsx`.
`vite build --mode demo` erzeugt ausschließlich statische Browser-Dateien.
Es gibt keinen separaten Nachbau der Screens und keinen Rust-/WASM-Build.
Änderungen an Shell, Views, Komponenten, Design-Tokens und CSS gehen damit
automatisch in beide Builds ein. Nur Demo-Hinweis und Fehlerbehandlung sind
Browser-Zusätze. Desktop- und Rust-Builds bleiben unabhängig.

## Start und Isolation

Die Website bietet einen Startbutton. Dieser öffnet `/demo/` in einem eigenen
Tab mit der vollständigen Anwendung, ohne eingebettete Mini-Vorschau.
Vor dem Start lädt die Website keine UI-, Font- oder Mock-Assets der Demo.
Die Desktop-Oberfläche benötigt derzeit mindestens 960 Pixel Breite; auf
kleineren Displays kann horizontal gescrollt werden.

Der Build-Schalter `__WEB_DEMO__` erzwingt den bestehenden `Transport`-Mock,
auch wenn ein Fenster vermeintliche Tauri-Globals anbietet. Native IPC-Imports
werden aus dem Demo-Bundle entfernt. Die zusätzliche CSP verbietet Netzwerk-
verbindungen (`connect-src 'none'`), Formulare, Plugins und fremde Ressourcen.
Die Demo enthält keine Treiber, Daemon-Binaries oder Zugangsdaten.
Ein Build-Audit lehnt native IPC- und bekannte Netzwerk-API-Referenzen sowie
die früher verwendeten Hardwaremarken/-modelle im fertigen JavaScript ab.
Das Audit ergänzt die CSP; es ersetzt keinen allgemeinen JavaScript-Sicherheitsaudit.

Die Simulation verwendet neutrale Gerätenamen und arbeitet im Arbeitsspeicher.
Effekte, Helligkeit, Layout, Profile, Kurven und Editoren verwenden dieselbe
RPC-/Slice-Grenze wie die echte Oberfläche. Geräteeinrichtung zeigt simulierte
Downloads, Verifikation, Fehler, Wiederholung und Abbruch. Es wird nichts
heruntergeladen oder installiert. Ein Neuladen setzt die Simulation zurück;
die vorhandene Einstellung für die Einführungstour kann im Browser bleiben.

Audio-/Bildschirmaufnahme, Makroausführung und native Diagnoseexporte sind
ausdrücklich nicht verfügbar. Die gemeinsamen Effektvorschauen sind CSS-
Darstellungen mit vereinfachter Animation für Rainbow Wave, Breathing,
Scanner und Colour Cycle, kein exakter Port des Rust-Effektrenderers.
Geschwindigkeit und globale Helligkeit beeinflussen die Vorschau;
reduzierte Bewegung wird berücksichtigt. Die Demo stellt
keine tatsächlichen FPS, Sensormessungen oder Hardware-Lichttests dar.

## Lokal bauen

Node.js 24 und npm verwenden:

```sh
npm ci --prefix apps/desktop
npm ci --prefix website
npm test --prefix apps/desktop
npm run build --prefix apps/desktop
npm run build:site --prefix website
npm run preview --prefix website -- --host 127.0.0.1
```

`website/dist` enthält die Website plus `demo/`. Der Desktop-Build bleibt in
`apps/desktop/dist`, der Browser-Build in `apps/desktop/dist-demo`.
`npm run build` im Website-Ordner allein baut weiterhin nur die Website.
Für die gesamte auslieferbare Website immer `build:site` verwenden.

Für einen Unterpfad muss `BASE_PATH` bei Website-Build **und** Vorschau gesetzt
werden, z. B. in PowerShell `$env:BASE_PATH='/FreeRGB/'`. Die Demo selbst nutzt
relative Asset-URLs und funktioniert sowohl unter einem Projektpfad als auch
auf einer eigenen Domain. Browser-Entwicklung nur der GUI: `npm run dev
--prefix apps/desktop` (weiterhin der bestehende Mock-Modus).

## Automatische Veröffentlichung

`.github/workflows/website.yml` prüft relevante Änderungen in Pull Requests.
Auf `main` sowie bei manuellem Start auf `main` wird nach erfolgreichem Build
ein gemeinsames Pages-Artefakt veröffentlicht. Änderungen nur an Rust-Backend,
Treibern oder Classic lösen keinen Website-Build aus. GUI-Quellen, Fixtures,
Build-Konfiguration, Lockfiles, Website, Branding-Assets und Website-Inhalte
sind im Pfadfilter enthalten.

Die Jobs führen Frontend-Tests, den unabhängigen Desktop-Frontend-Build und
den gesamten Website-/Demo-Build inklusive Audit aus. Erst danach startet
`deploy-pages`. Ein fehlgeschlagener Build ersetzt keine funktionierende
Veröffentlichung. Pull Requests erhalten keine Deployment-Berechtigungen.

Einmalige Hosting-Voraussetzung: Im Repository unter **Settings → Pages →
Build and deployment → Source** GitHub Actions auswählen. Der Workflow legt
die Hosting-Einstellung nicht selbst an. Standardpfad ist `/<Repositoryname>/`.
Für eine eigene Domain oder eine Benutzer-/Organisations-Root-Site die
Repository-Variable `WEBSITE_BASE_PATH` auf `/` setzen. Bestehende Domain-
Konfiguration muss zur gewählten Pages-Site passen.

Vite versioniert JS, CSS und Fonts über Inhalts-Hashes. HTML verweist auf die
aktuellen Hashes; kein Service Worker hält eine alte Demo fest. Pages legt
die HTTP-Cache-Header fest. `demo/version.json` enthält Commit-SHA und Bauzeit,
um veröffentlichte Versionen zuzuordnen. Die Anwendung verwendet bereits
geteilte Chunks und lazy geladene Views; erst der Startbutton lädt die Demo.

## Validierung und Grenzen

Lokal geprüft: Frontend-Tests einschließlich Session-Isolation, neutraler
Namen, Slice-Updates, Download-Abbruch und Fehler für nicht simulierte
Funktionen; Desktop-Frontend-Build; gesamter Build unter `/FreeRGB/` und
Artefakt-Audit. Browserprüfung mit der vollständigen Oberfläche und
simulierter Geräteeinrichtung. Die nativen Rust-Binaries wurden für diese
Frontend-Änderung nicht neu gebaut.

Zum Prüfen einer UI-Änderung beide Builds bauen und dieselbe View in Tauri
und `/demo/` öffnen. Nach der Veröffentlichung zusätzlich
`/demo/version.json` mit dem ausgelieferten Commit vergleichen. Ein echter
Pages-Deployment-Lauf und die Synchronität auf der öffentlichen Domain sind
erst nach Einrichtung von Pages und Übernahme des Branches nach `main`
verifizierbar; sie sind nicht durch einen lokalen Build bewiesen.
