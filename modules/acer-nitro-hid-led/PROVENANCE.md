# Provenance: freeRGB module cer-nitro-hid-led

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/AcerNitroHidKeyboardController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| DetectAcerHIDControllers | 	able::identify | VID 0x0CF2, PID 0x5130, usage page 0xFF5A, usage 0x01 |
| SetLEDs (device_id 0x65, Direct) | eport::direct | 11-byte feature report 0xA4, device id 0x65, mode 0x02, brightness 100, RGB, zone select 1 |
| RGBController_AcerNitroHidLed SetupZones | lib.rs::initialize | single LED, direct mode only; brightness/speed modes omitted |

Changes: shares the underlying Acer interface with the keyboard controller; this module models only the one-LED accessory. Test status: simulated only.
