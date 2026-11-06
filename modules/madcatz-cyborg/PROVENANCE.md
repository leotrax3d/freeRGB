# Provenance: freeRGB module `madcatz-cyborg`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/MadCatzCyborgController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `MadCatzCyborgControllerDetect.cpp` | `table::identify` | VID 0x06A3, PID 0x0DC5 |
| `MadCatzCyborgController.cpp` `Initialize` | `lib.rs::initialize` | 2-byte enable feature report 0xA1 0x00 |
| `MadCatzCyborgController.cpp` `SetColors` | `lib.rs::frame` | 9-byte feature report CMD_COLOR + RGB; intensity packet 0xA6 00 100 |

Changes: Direct mode only. Test status: simulated only.