# Sony gamepad provenance and audit

Upstream: OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/SonyGamepadController`; all nine controller files carry
GPL-2.0-or-later headers.

| Upstream path or device | Audit | Reason |
|---|---|---|
| `SonyDS4Controller.cpp:17-35` initialization | not ported | Requires feature report `0x02` and up to five input reads to distinguish USB from Bluetooth. |
| `SonyDS4Controller.cpp:61-125` reports | not ported | USB is 11 bytes; Bluetooth is 78 bytes with signed CRC32. The old module always emitted the USB form. |
| `RGBController_SonyDS4.cpp:54-99` one-LED direct lifecycle | simplified | Inventory/layout existed, but transport initialization and Bluetooth cleanup were absent. |
| DS4 `054c:05c4` | not ported | Full dynamic USB/Bluetooth path missing. |
| DS4 `054c:09cc` | not ported | Full dynamic USB/Bluetooth path missing. |
| DS4 receiver `054c:0ba0` | not ported | Full receiver path missing. |
| `SonyDualSenseController.cpp:18-29` initialization/cleanup | not ported | Bluetooth is selected by interface `-1`; the old module ignored it. |
| `SonyDualSenseController.cpp:54-129` reports | simplified | Old 48-byte packet omitted direct mode byte `1`, used the wrong default brightness byte, and had no 78-byte Bluetooth CRC path. |
| `RGBController_SonyDualSense.cpp:83-134` lightbar/player mapping | simplified | Six logical LEDs were represented, but only player-LED on/off bits and one lightbar RGB value exist upstream. |
| DualSense `054c:0ce6` | not ported | Complete USB/Bluetooth lifecycle missing. |
| DualSense Edge `054c:0df2` | not ported | Complete USB/Bluetooth lifecycle missing. |

Consequently `identify()` reports all five detector rows as unsupported and no
guessed packet builder remains compiled into the module.

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/SonyGamepadController/SonyDS4Controller/RGBController_SonyDS4.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDS4Controller/RGBController_SonyDS4.cpp) | GPL-2.0-or-later | Pol Rius (alpemwarrior) 24 Sep 2020 |
| `Controllers/SonyGamepadController/SonyDS4Controller/RGBController_SonyDS4.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDS4Controller/RGBController_SonyDS4.h) | GPL-2.0-or-later | Pol Rius (alpemwarrior) 24 Sep 2020 |
| `Controllers/SonyGamepadController/SonyDS4Controller/SonyDS4Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDS4Controller/SonyDS4Controller.cpp) | GPL-2.0-or-later | Pol Rius (alpemwarrior) 24 Sep 2020 |
| `Controllers/SonyGamepadController/SonyDS4Controller/SonyDS4Controller.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDS4Controller/SonyDS4Controller.h) | GPL-2.0-or-later | Pol Rius (alpemwarrior) 24 Sep 2020 |
| `Controllers/SonyGamepadController/SonyDualSenseController/RGBController_SonyDualSense.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDualSenseController/RGBController_SonyDualSense.cpp) | GPL-2.0-or-later | Flora Aubry 01 Jul 2022 |
| `Controllers/SonyGamepadController/SonyDualSenseController/RGBController_SonyDualSense.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDualSenseController/RGBController_SonyDualSense.h) | GPL-2.0-or-later | Flora Aubry 01 Jul 2022 |
| `Controllers/SonyGamepadController/SonyDualSenseController/SonyDualSenseController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDualSenseController/SonyDualSenseController.cpp) | GPL-2.0-or-later | Flora Aubry 01 Jul 2022 |
| `Controllers/SonyGamepadController/SonyDualSenseController/SonyDualSenseController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyDualSenseController/SonyDualSenseController.h) | GPL-2.0-or-later | Flora Aubry 01 Jul 2022 |
| `Controllers/SonyGamepadController/SonyGamepadControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/SonyGamepadController/SonyGamepadControllerDetect.cpp) | GPL-2.0-or-later | Pol Rius (alpemwarrior) 24 Sep 2020; Flora Aubry 01 Jul 2022; Yoan Berthelot 06 Mar 2025 |

Changes: ported to a Rust freeRGB module (process module under `modules/sony-gamepad/`); device/detector tables generated by `scripts/openrgb/port-sony-gamepad-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
