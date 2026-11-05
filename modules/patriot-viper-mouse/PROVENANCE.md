# Provenance: freeRGB module `patriot-viper-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/PatriotViperMouseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `PatriotViperMouseControllerDetect.cpp` | `table::identify` | VID 0x0C45, PID 0x7E18, interface 2, usage page 0xFF18, usage 0x01 |
| `PatriotViperMouseController.cpp` constructor | `lib.rs::initialize` | 64-byte feature init packet sent during initialize |
| `PatriotViperMouseController.cpp` `SetRGB` | `report::frames` | 7 feature packets (one per LED) with XOR-based checksum adjustment |

Changes: Direct mode only; LED names generic. Serial reads omitted. Test status: simulated only.