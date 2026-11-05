# Provenance: freeRGB module lenovo-motherboard-led

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/LenovoMotherboardController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectLenovoMotherboardControllers | 	able::identify | VID 0x17EF, PID 0xC955, usage page 0xFF89, usage 0xCC |
| SetMode (Static) | eport::direct | two 64-byte feature reports per LED, brightness max, speed min |
| RGBController LenovoMotherboard SetupZones | lib.rs::initialize | 2 LEDs direct mode only; DMI name and effects omitted |

Changes: Static color mode mapped to Direct; Wave/Breathing/etc. omitted; DMI name omitted. Test status: simulated only.
