# Reverse Engineering – Verfahren und Befunde

Kennzeichnung: **[B]** bestätigt (reproduzierbar belegt), **[W]** wahrscheinlich
(statisch belegt, nicht an Hardware geprüft), **[U]** unbekannt/offen.

## 1. Werkzeuge (Stand 2026-10-18)

| Werkzeug | Status | Nutzen |
|---|---|---|
| MCP „Reverse Engineer Anything“ | **nicht verfügbar** – in dieser Sitzung existiert kein solcher MCP-Server (ToolSearch ohne Treffer) | – |
| PowerShell / Git Bash | verfügbar | Dateisystem, Prozesse, Dienste, PnP-/HID-Enumeration (`Get-PnpDevice`, `Get-PnpDeviceProperty`) |
| Git 2.55 | verfügbar | Versionierung |
| Rust 1.99.1 (MSVC) + VS 2022 Build Tools | am 2026-10-18 nach Freigabe per winget installiert | Build, Tests, eigene Diagnose-CLI |
| `freergb discover` / `bragi probe` (eigene Werkzeuge) | verfügbar | HID-Inventar, Report-Descriptor-Auswertung, Read-only-Abfragen |
| USB-Mitschnitt (USBPcap/Wireshark) | **nicht installiert** | wäre für passive Mitschnitte nötig (Treiberinstallation → Zustimmung erforderlich) |
| Debugger/Disassembler (x64dbg, Ghidra) | nicht installiert | bisher nicht nötig, da Geräteprotokolle als JS-Plugins vorliegen |

Fehlende Fähigkeiten wurden **nicht** durch Annahmen ersetzt: Alle Protokollaussagen
stammen aus (a) statischer Lektüre der SignalRGB-Plugins, (b) SignalRGB-Logdateien,
(c) eigenen Read-only-Abfragen und (d) kontrollierten, reversiblen Farbtests.

## 2. SignalRGB-Bestandsaufnahme [B]

Version **2.5.80.0** (Build `53e4a349`), Squirrel-Installer.

| Pfad | Inhalt |
|---|---|
| `%LOCALAPPDATA%\VortxEngine\app-2.5.80\Signal-x64\SignalRgb.exe` (113 MB) | Hauptprogramm, Qt 6 / QML |
| `…\Signal-x64\Qt6*.dll`, `qml\` | Qt 6.12-Laufzeit, QtQuick-UI (546 `.qml`) |
| `…\Signal-x64\WebCore.dll`, `UltralightCore.dll` | Ultralight-HTML-Engine → Effekte sind HTML/JS-Seiten |
| `…\Signal-x64\Qt6WebEngineCore.dll` | Chromium 140 (Store/Webinhalte) |
| `…\Signal-x64\Qt6HttpServer.dll`, `Qt6WebSockets.dll` | lokale HTTP-/WebSocket-Schnittstelle **[W]** (Logs: „WebSocketTransport“) |
| `…\Signal-x64\git2.dll` | libgit2 → Plugin-/Add-on-Repos werden per Git geklont **[W]** (Cache enthält `.github/workflows`) |
| `…\Signal-x64\tesseract41.dll`, `leptonica` | OCR (Spielintegrationen) |
| `…\Signal-x64\nvapi64.dll` | NVIDIA-GPU-Beleuchtung über NvAPI-I2C |
| `…\post\CDM21228_Setup\` | FTDI-Treiberpaket (serielle Geräte) |
| `C:\ProgramData\WhirlwindFX\SignalRgb\service\SignalRgbService.exe` | Windows-Dienst `SignalRgb.Service`, LocalSystem, Autostart |
| `C:\Windows\System32\Drivers\SignalRgbDriver.sys` | Kernel-Treiber (läuft) – Low-Level-I/O für SMBus/SuperIO („SignalIO“ in Logs) |
| `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins\` | **341 Geräte-Plugins in JavaScript** (ES-Module) |
| `…\cache\addons\` | Community-Add-ons (QMK-Tastaturen, WLED, Netzwerkgeräte) |
| `…\cache\i2c_scan_results.json` | SMBus-Scanergebnis |
| `…\cache\system_info.json` | GPUs, RAM-Module |
| `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\Logs\` | Logs; Geräteplugins loggen Antwortpakete |
| `%USERPROFILE%\Documents\WhirlwindFX\{Effects,Plugins,Components,Macroscripts}` | Benutzerordner (leer) |

Prozessstruktur [B]: `SignalRgbLauncher.exe` → `SignalRgb.exe` (+ `QtWebEngineProcess.exe`),
getrennt davon `SignalRgbService.exe` (Dienst) und der Kernel-Treiber.

### Plugin-Modell [B]
Jedes Geräte-Plugin ist ein ES-Modul. Häufigste Exporte (Anzahl Plugins):
`Name/Publisher/Initialize/Render/Shutdown/ImageUrl` (338), `ControllableParameters` (336),
`VendorId/ProductId` (317), `Validate(endpoint)` (309), `LedNames/LedPositions` (266),
`Scan` (21, für Busgeräte), `SubdeviceController` (60).
Host-API `device.*`: `write`(914), `pause`(698), `send_report`(580), `color(x,y)`(301),
`read`(260), `set_endpoint`(186), `get_report`(94), `createSubdevice`, `setControllableLeds`,
`bulk_transfer`, `control_transfer`, `getHidInfo` u. a.
→ Ablauf: Host enumeriert HID, ruft `Validate` je Interface, dann `Initialize`, pro Frame
`Render` (liest Canvas per `device.color(x,y)`), beim Beenden `Shutdown(suspending)`.

Auffälligkeit [B]: Das Link-Hub-Plugin enumeriert seine Kindgeräte offenbar bei jedem
Render erneut – ~100 MB Logs in ~50 Minuten.

### Lizenz [U]
Im Plugin-Cache liegt keine Lizenzdatei. Daher wird **kein Plugin-Code übernommen**.
Übernommen werden nur funktionale Schnittstellenfakten (Befehlsbytes, LED-Indizes),
die für Interoperabilität nötig sind, in eigener Formulierung und mit Quellenangabe.

## 3. Reproduzierbares Verfahren

1. **Inventar:** `freergb discover --all` (HID) bzw. PnP-Abfrage (USB-Namen,
   `HID_DEVICE_UP:xxxx_U:yyyy`-Hardware-IDs).
2. **Plugin finden:** `Select-String` über `plugin_cdn` nach `0x<PID>`.
3. **Statische Analyse:** Plugin lesen, Befehle/Felder in `docs/PROTOCOLS.md` als [W] eintragen.
4. **Laufzeitbelege:** SignalRGB-Logs nach Gerät filtern (Thread-Name enthält `vid:pid:serial`).
5. **Simulation:** Protokoll in `freergb-protocols` implementieren, gegen Simulator
   (`MockTransport`) testen.
6. **Read-only an Hardware:** nur Ping/GetProperty (`freergb bragi probe`). Erwartete
   Werte (z. B. eigene PID) validieren das Antwortformat.
7. **Reversibler Schreibtest:** Farbe setzen, danach Hardware-Modus zurück und per
   Rücklesen prüfen; visuelle Bestätigung durch den Nutzer.
8. Erst dann Status `[B]` und `SupportLevel::Validated`.

Vor Hardwaretests muss SignalRGB beendet sein (sonst konkurrierende Zugriffe):
`Stop-Process -Name SignalRgbLauncher,SignalRgb` (Dienst bleibt unberührt).

## 4. Gefundene Hardware (dieses System) [B]

Siehe `docs/DEVICES.md`.
