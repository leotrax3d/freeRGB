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
| Aerox 3/5/9 (`SteelSeriesAerox*Controller`) | 15 | nicht portiert |
| Rival 3 Wireless (`SteelSeriesAeroxWirelessController`) | 1 | nicht portiert |
| Siberia / Arctis (`SteelSeriesSiberiaController`, `SteelSeriesArctis*Controller`) | 4 | nicht portiert |
| QCK (`SteelSeriesQCKMatController`) | 9 | nicht portiert |
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