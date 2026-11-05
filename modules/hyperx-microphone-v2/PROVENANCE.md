# Provenance: freeRGB module hyperx-microphone-v2

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/HyperXMicrophoneV2Controller.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectHyperXMicrophoneV2 | 	able::identify | VID 0x03F0, PID 0x02B5, interface 1, usage page 0xFF13 |
| SendDirect | eport::header, eport::color_packet | 64-byte header + 6 color packets (20 LEDs each, final 8) |
| RGBController... SetupZones matrix_map | lib.rs::initialize | 108 LEDs, width 12 height 9; Direct-only |

Changes: keepalive thread omitted (daemon provides frames); reads omitted; Save/effect modes omitted. Test status: simulated only.
