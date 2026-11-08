# MSI keyboard provenance and audit

Upstream: OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSIKeyboardController`; all ten source files are
GPL-2.0-or-later.

| Upstream path or device | Audit | Reason |
|---|---|---|
| `MSIKeyboard1565ControllerDetect.cpp:16-34` | ported (inventory only) | Exact VID/PID/usage detector is generated. |
| `MSIKeyboard1565Controller.cpp:23-63` | not ported | MS_1565 has distinct 64-byte mode and colour structures and feature-report sequence. |
| `RGBController_MSIMysticLightKB.cpp:46-191` | not ported | Device plus eight zones, mode-specific colours, speed and direction were replaced by one guessed LED. |
| MS_1565 (`1462:1601`) | not ported | Complete mode/zone/direct lifecycle missing. |
| `MSIMysticLigh64ControllerDetect.cpp:19-39` | ported (inventory only) | Three exact HID registrations are generated. |
| `MSIMysticLight64Controller.cpp:20-55` | not ported | Separate MSI_64 packet structure and feature report are absent. |
| `RGBController_MSIMysticLight64.cpp:46-160` | not ported | Four hardware zones and seven hardware modes are absent. |
| MS_1562 (`1462:1562`) | not ported | Complete MSI_64 lifecycle missing. |
| MS_1563 (`1462:1563`) | not ported | Complete MSI_64 lifecycle missing. |
| MS_1564 (`1462:1564`) | not ported | Complete MSI_64 lifecycle missing. |

The previous implementation was copied from an unrelated one-LED module. Its
invented packets were removed; all four detector rows now fail closed.

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565Controller.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565Controller.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565Controller.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565ControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIKeyboard1565Controller/MSIKeyboard1565ControllerDetect.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIKeyboard1565Controller/RGBController_MSIKeyboard1565.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIKeyboard1565Controller/RGBController_MSIKeyboard1565.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIKeyboard1565Controller/RGBController_MSIMysticLightKB.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIKeyboard1565Controller/RGBController_MSIMysticLightKB.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLigh64ControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLigh64ControllerDetect.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLight64Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLight64Controller.cpp) | GPL-2.0-or-later | T-bond 03 Apr 2020; Adam Honse 06 Mar 2021; Elchanan Haas 23 Aug 2022 |
| `Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLight64Controller.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIMysticLight64Controller/MSIMysticLight64Controller.h) | GPL-2.0-or-later | T-bond 03 Apr 2020; Adam Honse 06 Mar 2021; Elchanan Haas 23 Aug 2022 |
| `Controllers/MSIKeyboardController/MSIMysticLight64Controller/RGBController_MSIMysticLight64.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIMysticLight64Controller/RGBController_MSIMysticLight64.cpp) | GPL-2.0-or-later | T-bond 03 Apr 2020; Adam Honse 06 Mar 2021; Elchanan Haas 23 Aug 2022 |
| `Controllers/MSIKeyboardController/MSIMysticLight64Controller/RGBController_MSIMysticLight64.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/MSIKeyboardController/MSIMysticLight64Controller/RGBController_MSIMysticLight64.h) | GPL-2.0-or-later | T-bond 03 Apr 2020; Adam Honse 06 Mar 2021; Elchanan Haas 23 Aug 2022 |

Changes: ported to a Rust freeRGB module (process module under `modules/msi-keyboard/`); device/detector tables generated by `scripts/openrgb/port-msi-keyboard-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
