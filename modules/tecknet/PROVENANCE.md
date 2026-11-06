# Provenance: freeRGB module `tecknet`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/TecknetController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `TecknetControllerDetect.cpp` | `table::identify` | VID 0x04D9, PID 0xFC05, usage page 0xFFA0, usage 0x01 |
| `TecknetController.cpp` `SendUpdate` | `report::direct` | 16-byte feature report, RGB inverted (255-R), brightness high 0x03, speed 0 |
| `RGBController_Tecknet.cpp` | `lib.rs` | single "Logo" LED, Direct only |

Changes: brokered `send_feature`; serial reads omitted. Test status: simulated only.