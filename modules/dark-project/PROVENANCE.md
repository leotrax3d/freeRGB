# Provenance: freeRGB module `dark-project`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/DarkProject`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `DarkProjectControllerDetect.cpp` | `table::identify` | VID 0x195D, PID 0x2061, interface 2, usage page 0xFFC2, usage 0x04 |
| `DarkProjectKeyboardController.cpp` `SetLedsDirect` | `report::frames` | two 256-byte feature reports (RG at bytes 5/107, B at byte 5) via `packet_map[88]` |
| `RGBController_DarkProjectKeyboard.cpp` `SetupZones` | `lib.rs::initialize` | 87 LEDs "LED 0..86", single matrix zone (6x18) |
| `RGBController_DarkProjectKeyboard.cpp` `DeviceUpdateLEDs` | `lib.rs::frame` | send both feature reports per frame |

Changes: brokered `send_feature`; serial name reads omitted; only Direct mode (effects and save are not exposed). LED names generic (`LED n`) instead of upstream EN key names. Test status: simulated only.