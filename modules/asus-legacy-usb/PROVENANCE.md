# Provenance
Ported from OpenRGB `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/AsusLegacyUSBController`.

## Audit correction

OpenRGB has three separate device paths: Cerberus keyboard, Sagaris keyboard,
and Strix Claw. The Rust module only contained a generic one-colour shortcut
and did not port their initialization, layout, direct mode, or cleanup. All
three detector rows are **nicht portiert** and `identify()` returns
`supported=false`.

| Upstream device/path | Rust mapping | Status |
|---|---|---|
| ASUS Cerberus Mech / `AsusCerberusKeyboard` | detector table only | nicht portiert |
| ASUS Sagaris GK1100 / `AsusSagarisKeyboard` | detector table only | nicht portiert |
| ASUS ROG Strix Claw / `AsusStrixClaw` | detector table only | nicht portiert |

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/AsusCerberusKeyboardController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/AsusCerberusKeyboardController.cpp) | GPL-2.0-or-later | Mola19 28 May 2022 |
| `Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/AsusCerberusKeyboardController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/AsusCerberusKeyboardController.h) | GPL-2.0-or-later | Mola19 03 Mar 2021 |
| `Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/RGBController_AsusCerberusKeyboard.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/RGBController_AsusCerberusKeyboard.cpp) | GPL-2.0-or-later | Mola19 28 May 2022 |
| `Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/RGBController_AsusCerberusKeyboard.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusCerberusKeyboardController/RGBController_AsusCerberusKeyboard.h) | GPL-2.0-or-later | Mola19 03 Mar 2021 |
| `Controllers/AsusLegacyUSBController/AsusLegacyUSBControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusLegacyUSBControllerDetect.cpp) | GPL-2.0-or-later | define ASUS_CERBERUS_MECH_PID 0x2047 |
| `Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/AsusSagarisKeyboardController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/AsusSagarisKeyboardController.cpp) | GPL-2.0-or-later | Mola19 20 Aug 2023 |
| `Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/AsusSagarisKeyboardController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/AsusSagarisKeyboardController.h) | GPL-2.0-or-later | Mola19 20 Aug 2023 |
| `Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/RGBController_AsusSagarisKeyboard.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/RGBController_AsusSagarisKeyboard.cpp) | GPL-2.0-or-later | Mola19 20 Aug 2023 |
| `Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/RGBController_AsusSagarisKeyboard.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusSagarisKeyboardController/RGBController_AsusSagarisKeyboard.h) | GPL-2.0-or-later | Mola19 20 Aug 2023 |
| `Controllers/AsusLegacyUSBController/AsusStrixClawController/AsusStrixClawController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusStrixClawController/AsusStrixClawController.cpp) | GPL-2.0-or-later | Mola19 06 Aug 2022 |
| `Controllers/AsusLegacyUSBController/AsusStrixClawController/AsusStrixClawController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusStrixClawController/AsusStrixClawController.h) | GPL-2.0-or-later | Mola19 06 Aug 2022 |
| `Controllers/AsusLegacyUSBController/AsusStrixClawController/RGBController_AsusStrixClaw.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusStrixClawController/RGBController_AsusStrixClaw.cpp) | GPL-2.0-or-later | Mola19 06 Aug 2022 |
| `Controllers/AsusLegacyUSBController/AsusStrixClawController/RGBController_AsusStrixClaw.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusLegacyUSBController/AsusStrixClawController/RGBController_AsusStrixClaw.h) | GPL-2.0-or-later | Mola19 06 Aug 2022 |

Changes: ported to a Rust freeRGB module (process module under `modules/asus-legacy-usb/`); device/detector tables generated by `scripts/openrgb/port-asus-legacy-usb-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
