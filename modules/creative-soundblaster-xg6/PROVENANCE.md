# Provenance: freeRGB module creative-soundblaster-xg6

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/CreativeController/CreativeSoundBlasterXG6Controller.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| Detect... | 	able::identify | VID 0x041E, PID 0x3256, interface 4 |
| SetLedColor packets | eport::packet1/2/3 | three 64-byte output reports with exact offsets, brightness fixed 0xFF |
| RGBController setup | lib.rs::initialize | single Logo LED, direct only |

Changes: Direct mode only; no init/read; effects omitted. Test status: simulated only.
