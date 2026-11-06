# Provenance: freeRGB module `n5312a`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/N5312AController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `N5312AControllerDetect.cpp` | `table::identify` | VID 0x4E53, PID 0x5406, interface 1, usage page 0xFF01, usage 0x01 |
| `N5312AController::SendInit` | `lib.rs::initialize` | 8-byte init feature report 07 A0 |
| `N5312AController::SetColor` | `report::color` | feature report 07 0B 01 R G B 00 00 |
| `RGBController_N5312A::DeviceUpdateMode` | `lib.rs::frame` | Mode report 07 0A 02 01 01 01 speed brightness |

Changes: Direct static mode only; init in constructor adopted at initialize. Test status: simulated only.