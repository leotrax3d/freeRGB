# Provenance: freeRGB module `skydimo-hid`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/SkydimoHIDController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| Detect: `0x1A86:0xE316` | `table::identify` | VID/PID, usage page 0xFF00, usage 0x01 |
| `SkydimoHIDController::SetLEDs` | `report::frames` | CRC8 (poly 0x07) over first 63 bytes of each 64-byte packet; GRB ordering for all 49 LEDs; 3 data packets + end marker packet |
| `RGBController_SkydimoHID::SetupZones` | `lib.rs::initialize` | 49 LEDs in 7x7 grid |

Changes: Direct mode only; serial-name reads omitted; no init. Test status: simulated only.