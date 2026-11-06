# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSIMotherboardController/`. The detector inventory is generated
by `scripts/openrgb/port-msi-devices.mjs` from
`MSIMotherboardControllerDetect.cpp`.

The freeRGB port uses brokered HID feature reports and never reads a serial
number or a device path.

## Ported sub-controllers

### 162-byte controller (MSIMotherboard162Controller) - fully ported

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `MSIMotherboardControllerDetect.cpp` 0x52 probe (162/163 bytes) | `lib.rs initialize_162()` | same probe as 112; variant switch by answer length |
| `MSIMotherboard162Controller.cpp` constructor | `lib.rs initialize_162()` | save flag 0, onboard master colorFlags = custom color + sync onboard; board_configs table (PIDs 0x1720/0x7B12/0x7B17/0x7B85/0x7B92 10 LEDs, 0x7B18 6, 0x7B50 6, 0xB926 3, default zones_set0/10) |
| `MSIMotherboard162Controller.cpp` `GetZoneData()` | `report::zone_slot_162()` / `ZONE_OFFSETS_162` | j_rgb_2 at offset 151, J_RAINBOW_2 remapped to on_board_led_10 (141) |
| `MSIMotherboard162Controller.cpp` `SetMode()` (Direct = MSI_MODE_STATIC) | `report::set_mode_162()` | base part for zone<=13 (effect 1, 0x28 flags, custom color, clears sync bits), second part writes zone+1 for the onboard chain |
| `MSIMotherboard162Controller.cpp` `SetZoneColor()` | `report::set_zone_color_162()` | color = color2 = LED color; onboard master block additionally receives the first onboard LED color |
| `MSIMotherboard162Controller.cpp` `Update()` | `lib.rs frame_162()` | stateful 162-byte packet, LEDs per zone in reverse order like DeviceUpdateLEDs, save flag 0 |

### 112-byte controller (MSIMotherboard112Controller) - fully ported

Upstream routes all 124 detector rows through one runtime probe
(`DetectMSIMotherboardControllers`): a 0x52 feature-report get whose answer
length selects the hardware variant. The Rust port performs the same probe in
`initialize()` and fails closed for boards answering with the 162/185/761
layouts, so `identify()` reports `supported=true` for the rows while only the
112-byte lifecycle is implemented.

| Upstream file/function | Rust mapping | Behaviour |
|---|---|---|
| `MSIMotherboardControllerDetect.cpp` 0x52 probe | `lib.rs initialize_with_zones()` | `get_feature(0x52, 200)`; only 112/113-byte answers proceed |
| `MSIMotherboard112Controller.cpp` `ReadFwVersion()` | `report::fw_request()` / `fw_string()` | 64-byte writes `[01 B0 CC...]` (APROM) and `[01 B6 CC...]` (LDROM) + 64-byte reads; version = high/low nibble of byte 2 |
| `MSIMotherboard112Controller.cpp` `ReadSettings()` | `lib.rs` | 112-byte 0x52 get report after the firmware reads |
| `MSIMotherboardCommon.h` `FeaturePacket_112` / `ZoneData` | `report::ZONE_OFFSETS_112` / `direct_packet_112()` | Report id 0x52, 10-byte ZoneData blocks at offsets 1,11,21,31,41,51,61,71,81,91,101, save flag at 111 |
| `MSIMotherboard112Controller.cpp` constructor (zone_based_per_led_data defaults) | `report::direct_packet_112()` | effect = MSI_MODE_STATIC, speedAndBrightnessFlags = 10<<2 (0x28), colorFlags = custom color (0x80) for all four configured zones and onboard LEDs 1..6 |
| `MSIMotherboard112Controller.cpp` `SetLedColor()` + `GetZoneData()` | `report::zone_slot_for_led()` / `direct_packet_112()` | Direct color for zone value 1 (JRGB1), 8 (JRainbow1), 11 (JCorsair) and onboard values 13..18 (remap +1); value 19 drops (nullptr), onboard master keeps black color |
| `MSIMotherboard112Controller.cpp` `Update()` (direct) | `lib.rs frame()` | Exactly one 112-byte feature report per frame, save flag 0 |
| `RGBController_MSIMotherboard112.cpp` `SetupZones()` / LED names | `lib.rs` | Zones JRGB1, JRAINBOW1, JCORSAIR (1 LED each), "Onboard LEDs" (7); LED names `<zone> LED [n]` |

## Remaining sub-controllers (not ported)

| Upstream path | Status | Grund |
|---|---|---|
| `MSIMotherboard162Controller` | portiert | 162-byte 0x52 variant; stateful SetMode/SetZoneColor (STATIC effect, 100% brightness flags, custom color, onboard +1 remap) and Update(save=0) byte-exakt |
| `MSIMotherboard185Controller` | blockiert | (1) die beiden `MSI Mystic Light Common`/`X870` Zeilen auf VID 0x0DB0 extrahieren die echte PID aus der Seriennummer (freeRGB liest nie Seriennummern), (2) der `DIRECT_MODE_PER_LED`-Pfad versendet JRAINBOW/JCORSAIR-Reports mit 13 ms Sleeps (Modulprotokoll hat kein Timing), (3) ~90 Board-Konfig-Einträge mit Mixed-Mode-Remap; Aufwand/Nutzen und Regel-9-Gründe sprechen gegen einen Partial-Port, der nicht byte-exakt wäre |
| `MSIMotherboard761Controller` | blockiert | needs the per-zone 0x51/0x50 protocol and the DMI mainboard name (`dmi.getMainboard()`), which is outside the brokered HID the freeRGB module provides |

## Changes

- Only Direct mode is exposed; the 22 upstream hardware-effect modes, manual
  save-to-flash and onboard-profile selection are omitted by the module API.
- `DEVICE_TYPE_MOTHERBOARD` maps to the freeRGB `LightingController` kind.
- The 0x52 variant probe runs in `initialize()` (HID available), not in
  `identify()`; non-112 boards fail closed.
- No serial-number read.

## Test status

Simuliert geprüft, nicht auf Hardware bestätigt. Byte-exakt: 112-Byte-Direct-
Paket (alle 12 Zone-Blöcke), LED-Zone-Remap, Firmware-Requests; Fake-Gerät
Sequenz initialize -> frame -> shutdown mit Probe-/Firmware-/Settings-Replays.