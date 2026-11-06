# Provenance: freeRGB module `nzxt-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/NZXTMouseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `NZXTMouseControllerDetect.cpp` | `table::identify` | VID 0x1E71, PID 0x2100, usage page 0xFFCA, usage 0x01 |
| `NZXTMouseController.cpp` `SendFirmwareRequest` | `lib.rs::initialize` | feature/write report `43 81 00 01...`, then read until `43 86` received |
| `NZXTMouseController.cpp` `SetLEDs` | `report::direct` | 64-byte packet reorder: colors[2], colors[1], colors[0], colors[3..5] |

Changes: Direct mode only. Test status: simulated only.