# Provenance: freeRGB module `dream-cheeky`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/DreamCheekyController`.

| Upstream file | Rust function | Adopted behaviour |
|---|---|---|
| `DreamCheekyControllerDetect.cpp` | `table::identify` | VID 0x1D34, PID 0x0004 |
| `DreamCheekyController.cpp` (constructor) | `lib.rs::initialize` | Four 9-byte init packets, followed by Direct mode |
| `DreamCheekyController.cpp` `SetColor` | `report::color` | 9-byte write; RGB scaled 0..255 to 0..64 (`>>2`, +1 at 255) |
| `RGBController_DreamCheeky.cpp` | `lib.rs` | One LED "LED", accessory kind, Direct only |

Changes: brokered `write` instead of hidapi; serial/product reads omitted; no Save (device does not support saving). Test status: simulated only.