# Provenance: freeRGB module cer-nitro-hid-keyboard

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/AcerNitroHidKeyboardController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| SetLEDs (device_id 0x21, Direct mode) | eport::direct | 11-byte feature report, 4 zone packets with zone select 1<<zone |
| RGBController keyboard SetupZones | lib.rs::initialize | 4 backlight zones, Direct-only adapter |

Changes: shares underlying Acer interface with LED accessory; effects omitted. Test status: simulated only.
