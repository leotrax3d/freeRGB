# Reverse engineering – procedure and findings

Labels: **[C]** confirmed (reproducibly evidenced), **[P]** probable (static evidence,
not checked on hardware), **[U]** unknown/open.

## 1. Tools (as of 2026-10-18)

| Tool | Status | Use |
|---|---|---|
| MCP "Reverse Engineer Anything" | **not available** – no such MCP server exists in this setup (tool search found nothing) | – |
| PowerShell / Git Bash | available | file system, processes, services, PnP/HID enumeration (`Get-PnpDevice`, `Get-PnpDeviceProperty`) |
| Git 2.55, GitHub CLI | available | version control, private repo |
| Rust 1.99.1 (MSVC) + VS 2022 Build Tools | installed via winget after approval | build, tests, own diagnostic CLI |
| `freergb discover` / `freergb bragi probe` (own tools) | available | HID inventory, report descriptor parsing, read-only queries |
| USB capture (USBPcap/Wireshark) | **not installed** | needed for passive captures (driver install → approval required) |
| Debugger/disassembler (x64dbg, Ghidra) | not installed | not needed so far, device protocols ship as JS plugins |

Missing capabilities were **not** replaced by assumptions: every protocol statement
comes from (a) static reading of SignalRGB plugins, (b) SignalRGB log files,
(c) own read-only queries and (d) controlled, reversible colour tests.

## 2. SignalRGB inventory [C]

Version **2.5.80.0** (build `53e4a349`), Squirrel installer.

| Path | Content |
|---|---|
| `%LOCALAPPDATA%\VortxEngine\app-2.5.80\Signal-x64\SignalRgb.exe` (113 MB) | main program, Qt 6 / QML |
| `…\Signal-x64\Qt6*.dll`, `qml\` | Qt 6.12 runtime, QtQuick UI (546 `.qml`) |
| `…\Signal-x64\WebCore.dll`, `UltralightCore.dll` | Ultralight HTML engine → effects are HTML/JS pages |
| `…\Signal-x64\Qt6WebEngineCore.dll` | Chromium 140 (store/web content) |
| `…\Signal-x64\Qt6HttpServer.dll`, `Qt6WebSockets.dll` | local HTTP/WebSocket interface [P] (logs: "WebSocketTransport") |
| `…\Signal-x64\git2.dll` | libgit2 → plugin/add-on repos are cloned via Git [P] (cache contains `.github/workflows`) |
| `…\Signal-x64\tesseract41.dll`, `leptonica` | OCR (game integrations) |
| `…\Signal-x64\nvapi64.dll` | NVIDIA GPU lighting via NvAPI I2C |
| `…\post\CDM21228_Setup\` | FTDI driver package (serial devices) |
| `C:\ProgramData\WhirlwindFX\SignalRgb\service\SignalRgbService.exe` | Windows service `SignalRgb.Service`, LocalSystem, auto start |
| `C:\Windows\System32\Drivers\SignalRgbDriver.sys` | kernel driver (running) – low-level I/O for SMBus/SuperIO ("SignalIO" in logs) |
| `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\cache\plugin_cdn\main\Plugins\` | **341 device plugins in JavaScript** (ES modules) |
| `…\cache\addons\` | community add-ons (QMK keyboards, WLED, network devices) |
| `…\cache\i2c_scan_results.json` | SMBus scan result |
| `…\cache\system_info.json` | GPUs, RAM modules |
| `%LOCALAPPDATA%\WhirlwindFX\SignalRgb\Logs\` | logs; device plugins log response packets |
| `%USERPROFILE%\Documents\WhirlwindFX\{Effects,Plugins,Components,Macroscripts}` | user folders (empty) |

Process structure [C]: `SignalRgbLauncher.exe` → `SignalRgb.exe` (+ `QtWebEngineProcess.exe`),
separately `SignalRgbService.exe` (service) and the kernel driver.

### Plugin model [C]
Every device plugin is an ES module. Most common exports (number of plugins):
`Name/Publisher/Initialize/Render/Shutdown/ImageUrl` (338), `ControllableParameters` (336),
`VendorId/ProductId` (317), `Validate(endpoint)` (309), `LedNames/LedPositions` (266),
`Scan` (21, bus devices), `SubdeviceController` (60).
Host API `device.*`: `write` (914), `pause` (698), `send_report` (580), `color(x,y)` (301),
`read` (260), `set_endpoint` (186), `get_report` (94), `createSubdevice`,
`setControllableLeds`, `bulk_transfer`, `control_transfer`, `getHidInfo` and more.
→ Flow: host enumerates HID, calls `Validate` per interface, then `Initialize`; per frame
`Render` (reads the canvas via `device.color(x,y)`); on exit `Shutdown(suspending)`.

Observation [C]: the Link hub plugin apparently re-enumerates its children on every
render – ~100 MB of logs in ~50 minutes.

### Licence [U]
The plugin cache contains no licence file. Therefore **no plugin code is reused**.
Only functional interface facts (command bytes, LED indices) needed for
interoperability are used, in our own words and with the source documented.

## 3. Reproducible procedure

1. **Inventory:** `freergb discover --all` (HID) or PnP query (USB names,
   `HID_DEVICE_UP:xxxx_U:yyyy` hardware IDs).
2. **Find the plugin:** `Select-String` over `plugin_cdn` for `0x<PID>`.
3. **Static analysis:** read the plugin, add commands/fields to `PROTOCOLS.md` as [P].
4. **Runtime evidence:** filter SignalRGB logs per device (thread name contains `vid:pid:serial`).
5. **Simulation:** implement the protocol in `freergb-protocols`, test against a
   simulator (`MockTransport`).
6. **Read-only on hardware:** only ping/GetProperty (`freergb bragi probe`). Expected
   values (e.g. the device's own PID) validate the response format.
7. **Reversible write test:** set a colour, return to hardware mode and read it back;
   visual confirmation by the user.
8. Only then status [C] and `SupportLevel::Validated`.

Before hardware tests SignalRGB must be stopped (otherwise concurrent access):
`Stop-Process -Name SignalRgbLauncher,SignalRgb` (the service is left untouched).

## 4. Hardware found on this system [C]

See `DEVICES.md`.
