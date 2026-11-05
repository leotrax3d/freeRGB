# Provenance: freeRGB module `roccat-burst-core`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/RoccatController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `RoccatControllerDetect.cpp` | `table::identify` | VID 0x1E7D, PID 0x2DE6, interface 3, usage page 0xFF01, usage 0x01 |
| `RoccatBurstController` `SetupDirectMode` | `lib.rs::initialize` | control feature packet 0E 06 01 01 00 FF |
| `RoccatBurstController::SendColors` | `lib.rs::frame` | 11-byte feature packet 0D 0B R G B 00... |

Changes: Direct mode only (LED count 1). Test status: simulated only.