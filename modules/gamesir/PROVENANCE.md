# Provenance: freeRGB module `gamesir`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/GameSirController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `GameSirControllerDetect.cpp` | `table::identify` | VID 0x3537, PIDs 0x100F/0x1098, interface 2, usage page 0xFF7A, usage 0x01 |
| `GameSirController.cpp` `SetColor` | `report::direct` | 65-byte write, RGB + checksum at byte 10 |
| `RGBController_GameSir` constructor | `lib.rs::initialize` | single accessory LED "LED", no init |

Changes: Direct mode only; serial reads omitted. Test status: simulated only.