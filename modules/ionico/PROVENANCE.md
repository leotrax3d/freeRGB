# Provenance: freeRGB module `ionico`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/IonicoController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `IonicoControllerDetect.cpp` | `table::identify` | Keyboard VID 0x048D PID 0xCE00 usage 0xFF12:01; Light Bar VID 0x048D PID 0x6005 usage 0xFF03:01 |
| `IonicoController.cpp` `SetMode` | `lib.rs::initialize` | Direct SetMode report for each variant |
| `IonicoController.cpp` `SetColors` | `report::direct_keyboard` / `report::lightbar_write` | Keyboard: 4 feature reports `14 00 (i+1) RGB 00 00`; Light Bar: feature `12 00 ...`, 65-byte write (`R,B,G, ...`), feature `12 00 01 ...` |

Changes: Direct mode only. Test status: simulated only.