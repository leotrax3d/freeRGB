# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/AsusAuraCoreController/`. Detector data is generated from the
pinned detector source. The built-in ASUS Aura USB motherboard device
`0b05:19af` is deliberately omitted because freeRGB owns it in-tree.

The module uses brokered HID only, with no serial or path reads. The direct
0xEC/0x40 65-byte packet framing and 20-LED chunking are preserved for the
common Aura USB transport; controller-specific firmware effects remain outside
the frame API.

## Audit correction

The former claim treated a common packet as a complete family port. OpenRGB
has separate desktop and laptop Aura Core paths with different feature-report
sequencing, layouts, direct modes and cleanup. The Rust module does not port
those complete paths. All five generated detector rows are **nicht portiert**
and `identify()` returns `supported=false`.

| Upstream path | Devices | Rust mapping | Status |
|---|---:|---|---|
| Aura Core desktop/laptop registrations | 5 | generated detector table only | nicht portiert |

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreController.cpp) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 13 Apr 2020 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreController.h) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 13 Apr 2020 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreController/AsusAuraCoreControllerDetect.cpp) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 13 Apr 2020 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreController/RGBController_AsusAuraCore.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreController/RGBController_AsusAuraCore.cpp) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 13 Apr 2020 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreController/RGBController_AsusAuraCore.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreController/RGBController_AsusAuraCore.h) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 13 Apr 2020 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopController.cpp) | GPL-2.0-or-later | Chris M (Dr_No) 28 Jul 2022 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopController.h) | GPL-2.0-or-later | Chris M (Dr_No) 28 Jul 2022 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopControllerDetect.cpp) | GPL-2.0-or-later | Chris M (Dr_No) 28 Jul 2022 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopDevices.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopDevices.cpp) | no SPDX line in header | no copyright/author line in header |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopDevices.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/AsusAuraCoreLaptopDevices.h) | no SPDX line in header | no copyright/author line in header |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/RGBController_AsusAuraCoreLaptop.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/RGBController_AsusAuraCoreLaptop.cpp) | GPL-2.0-or-later | Chris M (Dr_No) 28 Jul 2022 |
| `Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/RGBController_AsusAuraCoreLaptop.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/AsusAuraCoreController/AsusAuraCoreLaptopController/RGBController_AsusAuraCoreLaptop.h) | GPL-2.0-or-later | Chris M (Dr_No) 28 Jul 2022 |

Changes: ported to a Rust freeRGB module (process module under `modules/asus-aura-core/`); device/detector tables generated by `scripts/openrgb/port-asus-aura-core-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
