# Geräte

Inventar dieses Systems (Windows 11 Pro 26300), erhoben 2026-10-18 per PnP und
`freergb discover`. Alle Einträge [B] belegt.

| Gerät | VID:PID | RGB-Schnittstelle | SignalRGB-Plugin | freeRGB-Status |
|---|---|---|---|---|
| CORSAIR K70 CORE RGB (ISO, FW 1.23.77) | 1b1c:1bfd | IF1 `FF42:0001` | Modern_Corsair/Corsair_Bragi_Device.js | **Validated** (Farbe, Per-Key-Stichprobe, Moduswechsel) |
| CORSAIR iCUE LINK System Hub | 1b1c:0c3f | IF0 `FF42:0001` | Modern_Corsair/Corsair_ICUE_Link_Hub.js | Identified |
| ↳ 3 × iCUE LINK RX Fan, AIO LINK Titan 360 RX, LCD | (über Hub) | – | – | Identified (Log) |
| CORSAIR iCUE LINK AIO LCD Screen Module | 1b1c:0c4e | `000C:0001` | Corsair/Corsair_ICUE_Link_LCD.js | Identified |
| Corsair Vengeance RGB DDR5 CMH32GX5M2B6000Z30 (2 Module) | SMBus 0x18, 0x1A | SMBus (PIIX4) | Corsair/Ram/Corsair_Dominator_Ram.js | Identified, zurückgestellt (Risiko) |
| ASUS AURA LED Controller (Mainboard) | 0b05:19af | IF2 `FF72:00A1` | Asus/Motherboard Controllers | Identified |
| Razer Cobra (Maus) | 1532:00a3 | – | Razer/Razer_Modern_Mouse.js | Identified |
| Logitech USB Receiver | 046d:c548 | – | – | nicht RGB-relevant |
| unbekannt, ohne USB-Interface | 1b1c:1bf0 | Tastatur/Maus-Collections | – | [U] vermutlich virtuelles HID-Gerät |
| GPUs: NVIDIA RTX 5060 Ti (Gigabyte), AMD RX 9060 XT, AMD iGPU | PCI | I2C/NvAPI | – | nicht untersucht |

## Validierungsprotokoll K70 CORE (2026-10-18)
1. `bragi probe` (read-only): VID/PID/FW/Layout/Pollrate korrekt.
2. `bragi color #ff0000 --seconds 20`: vom Nutzer als „komplett rot“ bestätigt.
3. `bragi keys …`: Esc, L-Ctrl, L-Shift, L-Win, Fn, ISO `<`, ISO `#`, R-Ctrl in
   jeweils eigener Farbe – vom Nutzer als „alles korrekt“ bestätigt (bestätigt auch
   Kanalreihenfolge RGB).
4. Nach jedem Test `mode = 1 (hardware)` zurückgelesen.
5. `bragi effect rainbow --fps 60`: 132 Frames/6 s = 21,9 FPS, fehlerfrei.
