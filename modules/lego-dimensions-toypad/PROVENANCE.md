# Provenance: freeRGB module `lego-dimensions-toypad`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/LegoDimensionsToypadBaseController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| Detect: `0x0E6F:0x0241` | `table::identify` | VID/PID match |
| `Activate` | `lib.rs::initialize` | 32-byte feature packet (hid_write) with checksum 0xF7 |
| `SetDirect` | `lib.rs::frame` + `report::direct` | 32-byte packet: `00 55 06 C0 02 zone R G B` + checksum at byte 9 |

Changes: Direct mode only. Test status: simulated only.