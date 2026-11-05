# Provenance: freeRGB module xpg-summoner-keyboard

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/XPGSummonerKeyboardController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectXPGSummonerControllers (interface 2, UP 0xFF01, usage 1) | 	able::identify | VID 0x125F, PID 0x9418 |
| SendInitialize | eport::INIT_REPORT + initialize | 265-byte init report |
| DeviceUpdateLEDs (brightness 100, frame_buf) | lib.rs::frame + report::chunk | 104 LEDs, hardware index mapping real_idx, two 265-byte chunks (256 + 248) |
| SetupZones matrix map (NA positions) | LED_VALUES array | non-NA positions; 104 valid LEDs |

Changes: effect modes, TerminateColor omitted; sleeps omitted; generic LED names. Test status: simulated only.
