# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/SteelSeriesController/`. Detector data is generated from the
pinned `SteelSeriesControllerDetect.cpp` and includes the callback family for
each VID/PID. freeRGB uses brokered HID and never reads serial numbers or paths.

## Ported sub-controllers

### Rival family (SteelSeriesRivalController) - 19 detector rows, ported

Direct mode only; every packet byte is taken from upstream. LED values, zone
names and per-device LED sets come from `RGBController_SteelSeriesRival.cpp
SetupZones()`.

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesRivalController.cpp` `send_usb_msg()` | `report::write()` | 0x00 report-id prefix |
| `SteelSeriesRivalController.cpp` `SetLightEffect()` | `report::set_light_effect()` | 9-byte [cmd,zone,effect] packet; Rival 700 sends nothing; Rival 600/650 keep zero cmd/zone (C++ switch fallthrough) |
| `SteelSeriesRivalController.cpp` `SetLightEffectAll()` | `report::set_light_effect_all()` | R100 zone 0; R300 zones 0+1; R650 zones 0x10..0x17; R600/R700 none |
| `SteelSeriesRivalController.cpp` `SetColor()` | `report::set_color()` | R100 [05 00 rgb]; R300 [08 zone+1 rgb]; otherwise delegates |
| `SteelSeriesRivalController.cpp` `SetRival600Color()` | `report::set_rival600_color()` | 7-byte write, `1 << zone_id` mask |
| `SteelSeriesRivalController.cpp` `SetRival650Color()` | `report::set_rival650_color()` | 4 x 60-byte packets (61 with report id) |
| `SteelSeriesRivalController.cpp` `SetRival700Color()` | `report::set_rival700_color()` | 578-byte buffer + report id = 579-byte feature report |
| `SteelSeriesRivalController.cpp` `SaveMode()` | `report::save_mode()` | 9-byte 0x09 packet (not used by the module API; kept as tested builder) |
| `RGBController_SteelSeriesRival.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | LED names/values and zone structure per device type |
| `RGBController_SteelSeriesRival.cpp` `DeviceUpdateMode()`/`DeviceUpdateLEDs()` | `lib.rs frame()` | Direct: effect-all packets then SetColor per LED with upstream led->value ids |
| `RGBController.h` effect modes | - | Only the Direct mode is exposed; Pulsate/hardware modes omitted |
| `GetFirmwareVersion()` / `GetSerialString()` | - | Not used (module protocol has no firmware/serial path) |

Device rows: DetectSteelSeriesRival100 PIDs 0x1702, 0x170C, 0x1814, 0x1816,
0x1729; DetectSteelSeriesRival300 PIDs 0x1710, 0x1714, 0x1394, 0x1716, 0x171A,
0x1392, 0x1718 (0x1710 Black Ops row is a duplicate PID/name); DetectSteelSeriesRival600
PIDs 0x1724, 0x172E; DetectSteelSeriesRival650 PIDs 0x172B, 0x1726;
DetectSteelSeriesRival700 PIDs 0x1700, 0x1730. All on VID 0x1038, interface 0.
Rival 3: DetectSteelSeriesRival3 PIDs 0x1824 and 0x184C, interface 3.
Rival 5: DetectSteelSeriesRival5 PID 0x183C, interface 0, usage page 0xFFC0,
usage 1.
Sensei: DetectSteelSeriesSensei PIDs 0x1832, 0x1834, 0x1722, 0x1720, 0x171E,
0x1736, interface 0.
Siberia: DetectSteelSeriesHeadset PID 0x1229, interface 3.
Arctis 5: DetectSteelSeriesArctis5 PIDs 0x1250 and 0x12AA, interface 5.
Arctis Nova 3: DetectSteelSeriesArctisNova3 PID 0x12EC, interface 4,
usage page 0xFFC0, usage 1.
Aerox 3: DetectSteelSeriesAerox3 PID 0x1836, interface 3, usage page 0xFFC0,
usage 1.
Aerox 5: DetectSteelSeriesAerox5 PID 0x1850, interface 3, usage page 0xFFC0,
usage 1.

### Aerox 5 wired (SteelSeriesAerox5Controller) - 1 detector row, ported

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesAerox5Controller.h` `aerox_5` | `lib.rs led_specs()` / `zones()` | Front/Middle/Rear/Reactive (zone ids 0..3), one zone each |
| `SteelSeriesAerox5Controller.cpp` `SendInit()` | `report::aerox3_init()` (shared, same `[00 2D]` packet) | 65-byte feature report |
| `SteelSeriesAerox5Controller.cpp` `SetColor()` | `report::aerox5_set_color()` | Zones 0..2: cmd 0x21, mask 0x01/0x02/0x04, RGB at 3/6/9; zone 3: cmd 0x26, RGB at 0x04 |
| `SteelSeriesAerox5Controller.cpp` `SetBrightness()` | `report::aerox5_set_brightness()` | `[00 23 level]` zero-padded to 65 bytes (upstream writes a 3-byte buffer with the 65-byte packet size); guarded on change |
| `SteelSeriesAerox5Controller.cpp` `Save()` | `report::aerox5_save()` | `[00 11 00...]` (tested builder) |

Changes: upstream sleeps 2 ms between SetColor/SetBrightness packets; the
module protocol has no sleeps, so packets are emitted back to back.

### Aerox 3/5/9 Wireless + Rival 3 Wireless (SteelSeriesAeroxWirelessController)
- 13 detector rows, ported

Rows: Aerox 3 Wireless (0x1838), Aerox 3 Wireless Wired (0x183A), Aerox 3
Wireless Gen 2 (0x1890), Aerox 5 Wireless/Wired/Destiny/Diablo (0x1852, 0x1854,
0x185C, 0x185E, 0x1860, 0x1862), Aerox 9 Wireless/Wired (0x1858, 0x185A),
Rival 3 Wireless (0x1830). All interface 3, usage page 0xFFC0, usage 1.

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesAeroxWirelessController.h` tables `aerox_3_wireless`, `aerox_9`, `rival_3_wireless` | `report::AEROX_WIRELESS_*_LEDS` / `lib.rs` | 3 / 4 (incl. Reactive) / 1 (Scroll Wheel value 1) LEDs, one zone each |
| `SteelSeriesAeroxWirelessController.cpp` `SendInit()` | `report::aerox_wireless_init()` | 64-byte feature report `[00 2D]` |
| `SteelSeriesAeroxWirelessController.cpp` `SetColor()` | `report::aerox_wireless_set_color()` | `[00 21 01 zone rgb@4..7]`; Aerox 9 zone 3 -> cmd 0x26 byte 3=0; Rival 3 Wireless -> `[00 13 zone rgb@3..6]` |
| `STEELSERIES_AEROX_WIRELESS_FLAG = 0b01000000` | byte 1 |= 0x40 | Applied only when `IsWireless()` (Rival 3 Wireless and *_Wired rows: no flag) |
| `SteelSeriesAeroxWirelessController.cpp` `WriteBuffer()` | `lib.rs frame()` | hid_read_timeout(250 ms) readback after every write in wireless mode |
| `SteelSeriesAeroxWirelessController.cpp` `SetBrightness()` | `report::aerox_wireless_set_brightness()` | `[00 23 level]`, level = 0x64/10 = 10, only on change |
| `SetLightEffectAll()` / `Save()` | - | Direct-only module: effect 0x05 (Rainbow) never emitted; Rival 3 Wireless ignores EEPROM save |

### QCK Prism mousemats (SteelSeriesQCKMatController) - 9 detector rows, ported

Rows: DetectSteelSeriesMousemat PIDs 0x150A, 0x150D, 0x151E, 0x1514, 0x151C,
0x1516, 0x1518, 0x151A, 0x1520 (interface 3).

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `RGBController_SteelSeriesQCKMat.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | Mat Bottom / Mat Top LEDs in one "Mousemat" zone (2 LEDs) |
| `SteelSeriesQCKMatController.cpp` `SetColors()` | `report::qck_set_colors()` | 525-byte feature report `[00 0E .. 02 .. rgb@5/11 .. FF 32 C8 @8, FF 32 C8 @14 01@19 01@1A .. 01@1C]` + 65-byte apply write `[00 0D]` |

### Aerox 3 wired (SteelSeriesAerox3Controller) - 1 detector row, ported

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesAerox3Controller.h` `aerox_3` | `lib.rs led_specs()` / `zones()` | Front/Middle/Rear (zone ids 0/1/2), one zone each |
| `SteelSeriesAerox3Controller.cpp` `SendInit()` | `report::aerox3_init()` | 65-byte feature report `[00 2D 00...]`, sent at construction |
| `SteelSeriesAerox3Controller.cpp` `SetColor()` | `report::aerox3_set_color()` | 65-byte write `[00 21 (1<<zone) rgb at 3+zone*3]` |
| `SteelSeriesAerox3Controller.cpp` `SetBrightness()` | `report::aerox3_set_brightness()` | 65-byte write `[00 23 level]`, guarded on change (upstream member is uninitialized; we send on the first frame) |
| `SteelSeriesAerox3Controller.cpp` `Save()` | `report::aerox3_save()` | `[00 11 00...]` (not used by module API; tested builder) |

### Arctis Nova 3 (SteelSeriesArctisNova3Controller) - 1 detector row, ported

Direct mode only (upstream Breathe/ColorShift/Off effects omitted).

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesArctisNova3Controller.h` report defines | `report::set_arctis_nova3_direct()` | 521-byte effect packets (0x06) + 64-byte select/apply packets |
| `SteelSeriesArctisNova3Controller.cpp` `SelectZone()` | `report::arctis_nova3_select()` (private helper) | `[06 A5 01/02 state]`, zone 0 -> 0x02 |
| `SteelSeriesArctisNova3Controller.cpp` `ApplyZone()` | `report::arctis_nova3_apply()` (private helper) | zone 0 -> `[06 09]`, zone 1 -> `[06 A3]` |
| `SteelSeriesArctisNova3Controller.cpp` `SetDirect()` | `report::set_arctis_nova3_direct()` | `[06 AA sel .. 98/3A 55 96 .. 01 .. 02 .. rgb FF]` + SelectZone(0x0A) + ApplyZone |
| `RGBController_SteelSeriesArctisNova3.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | One linear zone "Nova 3" with Right (index 0) and Left (index 1); zone_id = led index |

### Arctis 5 (SteelSeriesArctis5Controller) - 2 detector rows, ported

Direct mode only (upstream has Direct as the only mode).

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesArctis5Controller.h` `ARCTIS_5_REPORT_SIZE`/`ARCTIS_5_REPORT_ID` | `report::set_arctis5_color()` | 37-byte packets, report id 0x06 |
| `SteelSeriesArctis5Controller.cpp` `SetColor()` | `report::set_arctis5_color()` | 6 packets: pre 0x81 0x43 0x01 0x22/0x23, color [06 8A 42 00 20 41 00 rgb FF 32 C8 C8], apply [.. 20 41 08 zone 01], apply [.. 20 60 zone], apply [.. 20 05] |
| `RGBController_SteelSeriesArctis5.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | Left/Right single-LED zones (zone ids 0/1), kind Headset |
| `RGBController_SteelSeriesArctis5.cpp` `DeviceUpdateLEDs()` | `lib.rs frame()` (Arctis5 branch) | Per-zone SetColor sequence (6 writes each) |

### Siberia 350 (SteelSeriesSiberiaController) - 1 detector row, ported

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesSiberiaController.cpp` `send_usb_msg()` | `report::siberia_packet()` (private helper) | 16-byte framing `[01 00 cmd len payload...]` |
| `SteelSeriesSiberiaController.cpp` `SetColor()` | `report::set_siberia_color()` | Commands 0x95/0x80 0xbf, 0x80/0x52 0x20, 0x83/rgb, 0x93/0x03 0x80 |
| `RGBController_SteelSeriesSiberia.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | Single "Headset" LED, kind Headset |
| `RGBController_SteelSeriesSiberia.cpp` `DeviceUpdateLEDs()` | `lib.rs frame()` (Siberia branch) | Four 16-byte writes per frame |

### Sensei (SteelSeriesSenseiController) - 6 detector rows, ported

Direct mode only.

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `RGBController_SteelSeriesSensei.cpp` `SetupZones()` | `lib.rs led_specs()` / `zones()` | Logo (zone 0) and Scroll Wheel (zone 1), one LED each |
| `SteelSeriesSenseiController.cpp` `SetColor()` | `report::set_sensei_color()` | 65-byte packet + 0x00 report id; cmd 0x5B, static flag 0x13=01, colors 0x1C..0x21 |
| `SteelSeriesSenseiController.cpp` `Save()` | `report::sensei_save()` | `[00 59 00...]` (not used by module API; tested builder) |
| `RGBController_SteelSeriesSensei.cpp` `DeviceUpdateLEDs()` | `lib.rs frame()` | Direct: SetColor per zone, no effect packets |
| `SetLightEffect()` Breathing/Rainbow | - | Omitted (hardware effects out of scope) |

### Rival 5 (SteelSeriesRival5Controller) - 1 detector row, ported

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesRival5Controller.h` `rival_5` | `lib.rs led_specs()` / `zones()` | Scroll Wheel 0, Body Left/Right 1-4 (0x01-0x08), Logo 9; single "Whole Mouse" linear zone |
| `SteelSeriesRival5Controller.cpp` `UpdateLEDs()` | `report::set_rival5_direct()` | 64-byte feature report `[00 21 FF 03 rgb x10]` |
| `SteelSeriesRival5Controller.cpp` `SetBrightness()` | `report::set_rival5_brightness()` | `[00 23 tier]`, tiers 0x64/0x32/0x19/0x0C/0x00; only on change (initial 0xFF) |
| `RGBController_SteelSeriesRival5.cpp` `DeviceUpdateLEDs()` | `lib.rs frame()` (Rival5 branch) | Brightness (guarded) then Direct colors feature report |
| `SetLightEffectAll()` / `Save()` / `GetFirmwareVersion()` | - | Upstream no-ops; Reactive/Rainbow modes omitted (Direct only) |

### Rival 3 (SteelSeriesRival3Controller over SteelSeriesMouseController base) - 2 detector rows, ported

Direct mode only. Upstream shares `RGBController_SteelSeriesRival3` between
Rival 3, Aerox 3/5/9 and the Rival 3 Wireless (AeroxWireless) controller
classes; only the `SteelSeriesRival3Controller` rows
(`DetectSteelSeriesRival3`, PIDs 0x1824 and 0x184C, interface 3) are claimed by
this port. The wireless detector row (0x1830, usage page 0xFFC0) stays
unsupported until the AeroxWireless controller port.

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `SteelSeriesRival3Controller.h` `rival_3` | `lib.rs led_specs()` / `zones()` | Front 0x01, Middle 0x02, Rear 0x03, Logo 0x04; one single-LED zone each |
| `SteelSeriesRival3Controller.cpp` `SetLightEffectAll()` | `report::set_mouse_light_effect_all()` | 4-byte write `[00 06 00 effect]` |
| `SteelSeriesRival3Controller.cpp` `SetColor()` | `report::set_mouse_color()` | 8-byte write `[00 05 00 zone r g b brightness]` |
| `SteelSeriesMouseController.h` `STEELSERIES_MOUSE_BRIGHTNESS_MAX` / `STEELSERIES_MOUSE_EFFECT_DIRECT` | `report::MOUSE_BRIGHTNESS_MAX` / `MOUSE_DIRECT_VALUE` | 0x64 / 0x04 |
| `RGBController_SteelSeriesRival3.cpp` `DeviceUpdateLEDs()` | `lib.rs frame()` (Rival3 branch) | SetColor per zone first, then effect packet `[00 06 00 04]` |
| `GetFirmwareVersion()` | - | Not used (module protocol has no firmware path) |

### Remaining sub-controllers (not ported)

| Upstream path | Devices | Status |
|---|---:|---|
| Aerox wireless / Rival 3 Wireless (`SteelSeriesAeroxWirelessController`) | 13 | portiert |
| QCK (`SteelSeriesQCKMatController`) | 9 | portiert |
| Apex (`SteelSeriesApex*Controller`, old Apex, Apex M) | 19 | nicht portiert |

## Changes

- Only Direct mode is exposed; upstream Pulsate/hardware effect modes and
  Manual-Save are omitted by the module API.
- No firmware-version query or serial-number read.
- `identify()` reports `supported=true` only for the five Rival protocols above;
  all other inventoried rows fail closed with a reason string.

## Test status

Simuliert geprüft, nicht auf Hardware bestätigt. Byte-exact packet tests for
every Rival packet type plus full initialize -> frame -> shutdown fake-device
sequences for all five protocols (writes and feature reports checked
byte-for-byte).