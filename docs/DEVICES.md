# Devices

Inventory of the development system (Windows 11 Pro 26300), collected 2026-10-18 via
PnP and `freergb discover`. All entries confirmed.

| Device | VID:PID | RGB interface | SignalRGB plugin | freeRGB status |
|---|---|---|---|---|
| CORSAIR K70 CORE RGB (ISO, FW 1.23.77) | 1b1c:1bfd | IF1 `FF42:0001` | Modern_Corsair/Corsair_Bragi_Device.js | **Validated** (colour, per-key sample, mode switch, daemon) |
| CORSAIR iCUE LINK System Hub (FW 4.1.144) | 1b1c:0c3f | IF0 `FF42:0001` | Modern_Corsair/Corsair_ICUE_Link_Hub.js | Read-only probe + approved software-mode window (children, rpm, temps confirmed) |
| ↳ 3 × RX fan, Titan 360 RX AIO, LCD, LX fan, 2 × type 27 (unknown) | (via hub) | – | – | Identified (live child list) |
| CORSAIR iCUE LINK AIO LCD Screen Module | 1b1c:0c4e | `000C:0001` | Corsair/Corsair_ICUE_Link_LCD.js | Identified |
| Corsair Vengeance RGB DDR5 CMH32GX5M2B6000Z30 (2 modules) | SMBus 0x18, 0x1A | SMBus (PIIX4) | Corsair/Ram/Corsair_Dominator_Ram.js | Identified, deferred (risk) |
| ASUS AURA LED Controller (mainboard) | 0b05:19af | IF2 `FF72:00A1` | Asus/Motherboard Controllers | Identified |
| Razer Cobra (mouse) | 1532:00a3 | – | Razer/Razer_Modern_Mouse.js | Identified |
| Logitech USB Receiver | 046d:c548 | – | – | not RGB-relevant |
| unknown, no USB interface | 1b1c:1bf0 | keyboard/mouse collections | – | [unknown] probably a virtual HID device |
| GPUs: NVIDIA RTX 5060 Ti (Gigabyte), AMD RX 9060 XT, AMD iGPU | PCI | I2C/NvAPI | – | not investigated |

## Validation log
K70 CORE, 2026-10-18, SignalRGB stopped:
1. `bragi probe` (read-only): VID/PID/FW/layout/poll rate correct.
2. `bragi color #ff0000 --seconds 20`: user confirmed "fully red".
3. `bragi keys …`: Esc, L-Ctrl, L-Shift, L-Win, Fn, ISO `<`, ISO `#`, R-Ctrl each in its
   own colour – user confirmed "all correct" (also confirms RGB channel order).
4. After every test `mode = 1 (hardware)` was read back.
5. `bragi effect rainbow --fps 60`: 132 frames / 6 s = 21.9 FPS, no errors.
6. `freergbd --virtual-device`: K70 + virtual strip connected, `color.set` via API,
   measured 22 FPS, `daemon.shutdown` → mode 1 read back.
