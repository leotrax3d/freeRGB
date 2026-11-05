# Provenance: freeRGB module thermaltake-poseidon-z-rgb

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/ThermaltakePoseidonZRGBController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectPoseidonZRGBControllers | table::identify | VID 0x264A, PID 0x3006, interface 1, usage page 0xFF01 |
| SetLEDsDirect | report::red_grn/blue frames | 264-byte feature reports, red/green and blue planes |
| RGBController... SetupZones matrix_map | lib.rs::initialize | 104 LEDs, 23x6 matrix; Direct-only, upstream effects omitted |

Changes: keepalive/effect modes omitted; direct frames only; sleeps omitted. Test status: simulated only.
