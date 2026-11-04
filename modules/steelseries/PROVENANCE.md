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

### Remaining sub-controllers (not ported)

| Upstream path | Devices | Status |
|---|---:|---|
| Aerox 3/5/9 (`SteelSeriesAerox*Controller`) | 15 | nicht portiert |
| Rival 3 / Rival 3 Wireless / Rival 5 (`SteelSeriesRival3Controller`, `SteelSeriesRival5Controller`) | 4 | nicht portiert |
| Sensei (`SteelSeriesSenseiController`, incl. Rival 310 rows) | 6 | nicht portiert |
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