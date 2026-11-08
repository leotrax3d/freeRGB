# Lokale Sammlung: 25 WASM-Effekte

Die Sammlung liegt ausschließlich im Entwicklungskatalog `effects/development/catalog.json`.
Der offizielle Katalog, die drei eingebetteten Beispiele und der Veröffentlichungsworkflow
bleiben auf ihrem bisherigen Umfang. Alle neuen Pakete tragen Version `1.0.0`, Publisher
`freeRGB Team` (`leotrax3d`), API 1 und `permissions: []`. Die Artefaktnamen sind lokale
Release-Asset-Namen; es werden keine Download-URLs oder Veröffentlichungen angelegt.

Alle Effekte besitzen **Geschwindigkeit** (0,1–3; Standard 1) und **Helligkeit**
(0–1; Standard 0,8). Zusätzliche Regler stehen unten. Die vollständigen Grenzen,
Schrittweiten, Standards, Kategorien und Vorschaufarben stehen in `effects/collection.json`.
Der Build ergänzt tatsächliche Dateigrößen und SHA-256-Hashes im Entwicklungskatalog.

| Effekt | Verhalten | Zusätzliche Regler |
|---|---|---|
| Polarlicht | Mehrschichtige weich wandernde Farbbänder. | Bänder, Bandbreite, Farbton |
| Feuer | Flackernde Flammen über einer Glutzone. | Hitze, Unruhe, Seed |
| Gewitter | Unregelmäßige Blitze mit Nachleuchten über dunklem Hintergrund. | Blitzhäufigkeit, Nachleuchten, Seed |
| Regentropfen | Tropfen erzeugen sich ausbreitende Kreiswellen. | Tropfen, Wellenbreite, Seed |
| Sternenhimmel | Sterne mit unabhängiger Helligkeit und Lebensdauer. | Sterndichte, Lebensdauer, Seed |
| Meteorschauer | Versetzte Meteore mit gerichteten Schweifen. | Richtung, Streuung, Schweiflänge, Seed |
| Ozean | Überlagerte Tiefenwellen mit hellen Schaumkronen. | Tiefe, Schaum, Wellengröße |
| Lava | Langsam fließende verschmelzende heiße Farbflächen. | Flächengröße, Kontrast, Seed |
| Plasma | Ineinanderlaufende mathematische Farbwellen. | Wellenzahl, Farbton |
| Galaxie | Rotierende Spiralarme mit unabhängig funkelnden Sternen. | Spiralarme, Windung, Seed |
| Schwarzes Loch | Spiralförmiger Sog um einen dunklen Kern mit Außenring. | Kernradius, Sogwindung, Farbton |
| Konfetti | Bunte Partikel mit unabhängigen Geburtszeiten und Verblassen. | Dichte, Verblassdauer, Seed |
| Glühwürmchen | Langsam wandernde pulsierende Lichtpunkte. | Dichte, Leuchtgröße, Seed |
| Neonröhren | Segmentierte Röhren mit zeitversetztem Startflackern. | Segmente, Startflackern, Farbton |
| Cyberpunk | Harte Farbflächen mit Lauflichtern und seltenen Störungen. | Störanteil, Lichtbahnen, Seed |
| Matrixregen | Fallende Lichtspuren mit wechselnder Länge. | Spurlänge, Spaltendichte, Seed |
| Radar | Rotierender Suchstrahl erhellt nachleuchtende Ziele. | Strahlbreite, Nachleuchten, Seed |
| Herzschlag | Rhythmischer Doppelpuls mit einstellbarer Ruhephase. | Ruheanteil, Pulsschärfe, Farbton |
| Atemlicht | Weiche Helligkeitskurve mit optionalem Farbwechsel. | Atemkurve, Farbwechsel, Farbton |
| Pendel | Ein Lichtpunkt schwingt mit weichen Umkehrpunkten. | Lichtbreite, Richtung, Farbton |
| Kaleidoskop | Rotierende Farbmuster mit einstellbarer Spiegelsymmetrie. | Symmetrie, Musterdichte, Farbton |
| Schachbrett | Wandernde wechselnde Felder mit Überblendung. | Felder, Überblendung, Richtung |
| Farbfluss | Mehrere Palettenfarben fließen entlang einer wählbaren Richtung. | Richtung, Farbzyklen, Farbton |
| Feuerwerk | Aufsteigende Raketen zerfallen in ausglühende Funken. | Funken, Explosionsgröße, Seed |
| Reaktionsdiffusion | Zwei reagierende diffundierende Stoffe bilden organische Muster. | Zufuhr, Abbau, Seed |

## Lokal ausprobieren (Windows / PowerShell)

Im Wurzelverzeichnis des Repositorys:

```powershell
# Bereits kompilierte Pakete sind eingecheckt. Neubau bei Quelländerungen:
rustup target add wasm32-unknown-unknown
python scripts/build-effects.py --collection
```

Wenn `python` nicht im PATH ist, steht in dieser Codex-Installation dieser Interpreter bereit:

```powershell
& "$env:USERPROFILE\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe" scripts/build-effects.py --collection
```

Beende einen eventuell laufenden FreeRGB-Daemon zunächst selbst, damit die Desktop-App
mit dem Entwicklungsdaemon am normalen lokalen Endpunkt verbunden wird. Starte dann
im Repository-Hauptverzeichnis:

```powershell
cargo run -p freergb-daemon --bin freergbd --locked -- --no-hardware --virtual-device --no-state --no-modules --effect-dev-directory "$PWD\effects\development"
```

In einem zweiten Terminal im gleichen Worktree:

```powershell
cd apps/desktop
npm ci  # Nur falls die Abhängigkeiten fehlen
npm run tauri -- dev
```

Im **Marketplace** „Check for updates“ wählen. Der lokale Katalog enthält 28 Einträge:
drei bisherige Beispiele plus 25 neue Effekte. Effekt installieren und „Open in Lighting“
wählen. Die Regler erscheinen im **Lighting-Tab**; dort Parameter verändern und anwenden.
Zur Deinstallation zuerst einen anderen Effekt aktivieren. Die farbigen Karten sind
statische Vorschaufarben, keine simulierte WASM-Ausgabe. Der Entwicklungsmodus verwendet
ausschließlich lokale Dateien und verbindet Bewertungen/Downloadzählung nicht mit Appwrite.
`--no-state` verwendet einen temporären Paketbestand; nach einem Neustart erneut installieren.
`--no-hardware` und `--virtual-device` erlauben das Ausprobieren ohne Zugriff auf echte Geräte.
Die Gastmodule selbst erhalten auch beim regulären Hardwarebetrieb keine Hardwareberechtigung.

Nur `npm run dev` im Browser zeigt den vorhandenen Demo-Transport; dieser kann keine
WASM-Pakete installieren. Für die vollständige Kette die native Tauri-App verwenden.

## Implementierung und Grenzen

- Statische RGB- und Feldpuffer, kein Heap/Allocator, keine Importe, kein WASI,
  keine Datei-, Netzwerk-, Audio-, Bildschirm- oder Geräteschnittstellen.
- Neue Gäste deklarieren 256 KiB festen linearen Speicher einschließlich 64 KiB Stack.
  Paketgrenze 512 KiB und Hostbudget von 2.000.000 Fuel pro Frame bleiben unverändert.
- Prozedurale Felder höchstens 32×16, bilinear auf die angefragte Oberfläche bis 128×64
  interpoliert. Auf einem einzelnen LED-Pixel wird das Feld gemittelt; bei sehr kurzen
  1D-Oberflächen werden Teilbereiche gemittelt. Vertikale 1D-Oberflächen nutzen dieselbe
  Längskoordinate. Formen verlieren auf wenigen LEDs zwangsläufig räumliche Details.
- Die vorhandene Engine rendert eine gemeinsame Oberfläche und bildet diese über die
  Gerätezonen auf LED-Positionen ab. Keine neue Layout- oder Geräteberechtigung.
- Zufall wird ausschließlich aus Seed, Raumkoordinaten und verstrichener Zeit berechnet.
  Partikelzahlen sind begrenzt (vier Meteore, maximal sechs Tropfen und 24 Feuerwerkfunken);
  Stern-, Konfetti-, Glühwürmchen- und Matrixfelder verwenden begrenzte Zellverfahren.
- Zeitbasierte Animation, keine Anzahl von Frames als Uhr. Lange Zeitwerte werden nach
  65.536 effektiven Sekunden zyklisch reduziert, um Gleitkommapräzision zu erhalten.
- Reaktionsdiffusion: echte Gray-Scott-Schritte auf einem toroidalen 16×8-Gitter, acht
  Schritte je effektiver Sekunde. Eine deterministische Wachstumstrajektorie wird innerhalb
  eines achtsekündigen Zyklus vorwärts und rückwärts abgespielt. Beliebiges Springen benötigt
  höchstens 31 Schritte; kein unbegrenztes Aufholen nach Pausen. Dies ist eine begrenzte
  organische Animation und keine hochauflösende, dauerhaft fortlaufende Simulation.
- Sinus, Winkel und Wurzeln werden importfrei angenähert; ausgelegt auf LED-Ausgabe.
- Keine reale Hardware oder interaktive native Fensterabnahme in den automatisierten Tests.

## Reproduzierbare Prüfungen

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cd apps/desktop
npm test
npm run build
cd ../..
cargo build -p freergb-daemon --bin freergbd --locked
./scripts/test-effect-collection.ps1
```

Die Sammlungstests laden die echten WASM-Dateien, installieren alle Pakete über den lokalen
Dateitransport, verifizieren Metadaten/Hashes, prüfen mehrere Frames, Einzel-LEDs, kurze
horizontale/vertikale Streifen, ungerade Flächen, 128×64 und den 320×200-Hostcanvas.
Jeder Regler wird auf sichtbare Wirkung sowie Minimum, Maximum und einen Zwischenwert
geprüft, zusätzlich kombinierte Grenzen und vollständiges Abdunkeln. Zeitabläufe bei 8,
30 und 60 FPS werden gegen direkt gerenderte Referenzzeitpunkte verglichen.
Vorhandene Hosttests prüfen Import-/Startverbote, Speicher-/Paket-/Fuel-Limits, Traps,
ungültige Puffer, Manipulationen und Katalog-Rollback. Die Daemon-Integration prüft alle
25 Effekte über `marketplace.install`, `effects.set`/`effects.get` und Deinstallation.

Das PowerShell-Prozess-Smoke startet einen eigenen Daemon mit zufälligem isoliertem
Named-Pipe-Endpunkt, virtueller Ausgabe und temporärem Zustand. Es prüft alle 25 Effekte
mit Standard- und Maximalparametern und beendet nur den selbst gestarteten Prozess.
Der lokale Entwicklungsmanager legt **keinen Community-Client** an, auch wenn produktive
Appwrite-Einstellungen vorhanden sind. Keine Testbewertung und kein Testdownload werden
an den echten Dienst gesendet.

Für visuelle Prüfung der tatsächlichen WASM-Frames:

```powershell
cargo run -p freergb-effect-host --example collection_preview --locked
```

Dies schreibt je Effekt vier PPM-Bilder nach `target/effect-previews`.

## Pr�fergebnis dieses Stands

- Alle 28 WASM-Programme gebaut; lokale SHA-256-/Gr��enpr�fung und Installation bestanden.
- Alle 25 neuen Effekte: Layout-/Mehrframe-/Regler-/Grenzpr�fungen und identische Ausgabe
  bei unterschiedlichen Framefolgen bestanden. Gast-ABI lehnt ung�ltige Dimensionen,
  negative/nicht endliche Zeit und Speicherwachstum ab.
- `cargo test --workspace --locked` bestanden; acht bereits vorhandene Spezialtests bleiben
  wie vorgesehen ignoriert (explizite Hardware-/Helper-Voraussetzungen).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` bestanden.
- Native Tauri-Tests (neun) und Tauri-Clippy ohne Warnungen bestanden.
- Frontend: 78 Tests bestanden; TypeScript und Vite-Produktionsbuild bestanden.
- Isoliertes Windows-Prozess-Smoke f�r Installation, Aktivierung, Rendering, �nderung und
  Deinstallation aller 25 Effekte bestanden. Kein produktiver Community-Client.
- Vier tats�chliche WASM-Frames je Effekt visuell als Bild�bersicht gepr�ft.

Der Prozess-Test deckte au�erdem API-Starvation bei verpasstem Framebudget im Debug-Build
auf: Die Render-Schleife gibt jetzt zwischen Frames mindestens eine Millisekunde frei.
Damit k�nnen API-Anfragen den Engine-Mutex auch bei langsamen Frames erreichen; das
Sandbox-Fuelbudget wird dadurch nicht erh�ht.
