# Provenance: freeRGB module occat-burst-pro-air

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/RoccatController (Burst Pro Air).

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectRoccat... | 	able::identify | VID 0x1E7D, PID 0x2CA6, interface 0, usage page 0x0001, usage 2 |
| SetDirectMode | eport::direct | 30-byte feature report, report ID 0x06, RGB entries per LED with marker bytes |
| SetupZones | lib.rs::initialize | 4 LEDs (Scroll Wheel, Logo, Left/Right button), Direct only |

Changes: hardware effects, SetModeValues, save and reads omitted. Test status: simulated only.
