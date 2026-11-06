# Provenance: freeRGB module `gigabyte-aorus-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/GigabyteAorusMouseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `GigabyteAorusMouseControllerDetect.cpp` | `table::identify` | VID 0x1044, PID 0x7A40, interface 3, usage page 0xFF01, usage 0x01 |
| `GigabyteAorusMouseController.cpp` `SendDirect` | `report::direct` | 8-byte feature report `CD 00 R G B 00 00 00` |
| `RGBController_GigabyteAorusMouse` constructor | `lib.rs::initialize` | single LED, no init |

Changes: Direct mode only. Test status: simulated only.