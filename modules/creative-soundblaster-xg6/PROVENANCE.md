# Provenance: freeRGB module creative-soundblaster-xg6

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/CreativeController/CreativeSoundBlasterXG6Controller.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| Detect... | 	able::identify | VID 0x041E, PID 0x3256, interface 4 |
| SetLedColor packets | 
eport::packet1/2/3 | three 64-byte output reports with exact offsets, brightness fixed 0xFF |
| RGBController setup | lib.rs::initialize | single Logo LED, direct only |

Changes: Direct mode only; no init/read; effects omitted. Test status: simulated only.

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5Controller_Windows.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5Controller_Windows.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5Controller_Windows.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5Controller_Windows.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5ControllerBase.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5ControllerBase.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5ControllerDetect_Windows.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/CreativeSoundBlasterAE5ControllerDetect_Windows.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/RGBController_CreativeSoundBlasterAE5_Windows.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/RGBController_CreativeSoundBlasterAE5_Windows.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterAE5Controller/RGBController_CreativeSoundBlasterAE5_Windows.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterAE5Controller/RGBController_CreativeSoundBlasterAE5_Windows.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6Controller.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6Controller.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6Controller.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6ControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterXG6Controller/CreativeSoundBlasterXG6ControllerDetect.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterXG6Controller/RGBController_CreativeSoundBlasterXG6.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterXG6Controller/RGBController_CreativeSoundBlasterXG6.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CreativeController/CreativeSoundBlasterXG6Controller/RGBController_CreativeSoundBlasterXG6.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CreativeController/CreativeSoundBlasterXG6Controller/RGBController_CreativeSoundBlasterXG6.h) | GPL-2.0-or-later | no copyright/author line in header |

Changes: ported to a Rust freeRGB module (process module under `modules/creative-soundblaster-xg6/`); device/detector tables generated by `scripts/openrgb/port-creative-soundblaster-xg6-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
