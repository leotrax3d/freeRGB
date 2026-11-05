# Provenance: freeRGB module `aoc-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/AOCMouseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `AOCMouseControllerDetect.cpp` | `table::identify` | VID 0x3938, PID 0x1179, interface 1, usage page 0xFF19, usage 0xFF19 |
| `AOCMouseController.cpp` `SendDirect` / `SendPacket` | `report::direct` | 60-byte feature report with fixed constants and RGB at 0x33/0x34/0x35 and 0x36/0x37/0x38 |
| `RGBController_AOCMouse::SetupZones` | `lib.rs::initialize` | Logo + Scroll Wheel LEDs |

Changes: Direct mode only. Test status: simulated only.