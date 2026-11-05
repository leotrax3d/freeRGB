# Provenance: freeRGB module `aoc-mousemat`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/AOCMousematController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `AOCMousematControllerDetect.cpp` | `table::identify` | VID 0x3938, PID 0x1162, interface 1, usage page 0xFF19, usage 0xFF19 |
| `AOCMousematController::SendDirect` | `report::direct` | 32-byte feature packet with constant bytes as upstream |
| `RGBController_AOCMousemat` constructor | `lib.rs::initialize` | single Mousemat LED |

Changes: Direct mode only; serial reads omitted. Test status: simulated only.