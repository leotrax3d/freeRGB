# Provenance: freeRGB module `lexip-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/LexipMouseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `LexipMouseControllerDetect.cpp` | `table::identify` | VID 0x04D8, PID 0xFD0A, interface 0, usage page 2, usage 0x01 |
| `LexipMouseController.cpp` `SetDirect` | `report::direct` | 64-byte write (`00 24 01 R G B 00 64 80 00...`) |
| `RGBController_LexipMouse` constructor | `lib.rs::initialize` | single LED, no init |

Changes: Direct mode only; serial reads omitted. Test status: simulated only.