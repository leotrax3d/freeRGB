# Provenance: freeRGB module `clevo-lightbar`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/ClevoLightbarController`.

| Upstream file/function | Rust function | Adopted behaviour |
|---|---|---|
| `ClevoLightbarControllerDetect.cpp` | `table::identify` | VID 0x048D, PID 0x7001, usage page 0xFF03, usage 0x02 |
| `ClevoLightbarController.cpp` `SetColor` | `report::color` | feature report `[14 00 01 R G B 00 00]` |
| `ClevoLightbarController.cpp` `SetMode` | `report::mode_direct` | feature report `[08 22 01 06 64 01 00 00]` (mode 1, brightness 100, speed default 5 inverted to 6) |
| `RGBController_ClevoLightbar.cpp` `DeviceUpdateLEDs` | `lib.rs::frame` | SetColor then SetMode each frame |
| `RGBController_ClevoLightbar.cpp` `SetupZones` | `lib.rs::initialize` | single "Lightbar" LED |

Changes: brokered `send_feature`; serial/product/firmware reads omitted; Off/AutoSave and effects not exposed (Direct only). Test status: simulated only.