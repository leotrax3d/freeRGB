# OpenRGB controller-family porting inventory

Inventory of the OpenRGB tree at commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
The classification is based on the controller directories and detector/transport
calls in that checkout. Only USB-HID families are eligible for freeRGB modules;
SMBus, I2C, PCI, GPU and driver-backed families remain candidates until the
module protocol is deliberately extended.

## Priority

| Priority | Family / proposed module | Transport | Size | Why |
|---:|---|---|---:|---|
| 1 | `RedragonController` / `redragon-mouse` | USB HID feature reports | 5 files / 398 lines | Fixed one-LED layout, 11 detectors, no shared dependency; now ported |
| 2 | `ThermaltakeRiingController` / `thermaltake-riing` | USB HID write/read | 13 files / 1,267 lines | Direct HID protocol, but standard Riing sizes are user-configured in OpenRGB |
| 3 | `LuxaforController` / `luxafor` | USB HID write | 5 files / 446 lines | Small fixed protocol; hardware confirmation is unavailable |
| 4 | `ThingMController` / `thingm-blink` | USB HID feature reports | 5 files / 272 lines | Small protocol, but OpenRGB's two-LED model needs device confirmation |
| 5 | `CorsairPeripheral(V2)` | USB HID feature reports | 13 / 13 files | Useful family, but overlaps existing built-in Corsair support and needs a conflict decision |
| 6 | `SteelSeriesController` | USB HID | 64 files / 6,674 lines | Many products and shared layout/protocol paths; high porting risk |
| 7 | `AsusAuraUSBController` | USB HID | 55 files / 11,392 lines | HID-capable but mostly motherboard/ARGB paths excluded by current policy |
| 8 | `HIDLampArrayController` | USB HID feature reports | 6 files / 985 lines | Requires report-descriptor parsing and dynamic lamp metadata, not available in module API v1 |

The upstream tree contains many additional detector directories. Families that
contain `SMBus`, `I2C`, PCI, GPU, SuperIO, serial/network transport or vendor
driver access are excluded from this USB-HID queue. Examples include
`CorsairDRAMController`, `ASRockSMBusController`, GPU controller directories,
`MSIMotherboardController`, `GigabyteSuperIORGBController`, `LinuxLEDController`,
and network/serial families such as `PhilipsHueController` and
`SkydimoSerialController`.

## Device and license notes

All controller source files used from OpenRGB carry an SPDX header. The
Redragon port is GPL-2.0-or-later and is isolated under `modules/`; no upstream
code is copied into `crates/`. No SignalRGB code is used. The pinned commit is
recorded in the module metadata, provenance and generated device table.

The development inventory in `docs/DEVICES.md` has no Redragon, Luxafor,
ThingM or Thermaltake hardware. Those modules are therefore simulation-tested
only until the user explicitly watches a hardware test. Existing Corsair and
ASUS devices are intentionally not claimed by this new module.

## Not ported in this pass

* Redragon effect modes, profile writes and persistent-save semantics are not
  exposed: freeRGB offers Direct only, while the direct color and apply packet
  sequence is preserved.
* No serial number, path or descriptor is read or reported.
* Thermaltake Quad/Trio, dynamic LampArray metadata, keyboard layout managers,
  receiver pairing and all non-HID transports remain unsupported.

## Status

`redragon-mouse`: ported, report builders and initialize/frame/shutdown sequence
simulated, matching and layout tests included; not confirmed on hardware.

## Continuation status

* 	hermaltake-riing: **blocked** — OpenRGB's standard Riing layout is manually configured per header (0–20 LEDs), but module protocol v1 has no configuration/descriptor exchange. Claiming a fixed layout would violate the complete-port and behaviour-preservation rules. Quad/Trio use separate protocols and remain separate candidates.

