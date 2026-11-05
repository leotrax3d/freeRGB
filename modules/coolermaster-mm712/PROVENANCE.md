# Provenance: freeRGB module `coolermaster-mm712`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/CoolerMasterController` (CMMM712).

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `CMMM712ControllerDetect.cpp` | `table::identify` | VID 0x2516, PID 0x0169, interface 3, usage page 0xFF0A, usage 0x02 |
| `CMMM712Controller.cpp` commands | `lib.rs::initialize` | sequence normal/mode/color status, then direct init 00 5A 81 02 |
| `CMMM712Controller::SetColors` | `lib.rs::frame` | 00 5A 81 03 R G B (65-byte) |

Changes: Direct mode only. Test status: simulated only.