# Provenance: freeRGB module `corsair-k65-mini`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/CorsairPeripheralController` (CorsairK65MiniController and RGBController_CorsairK65Mini).

| Upstream function | Rust module function | Adopted behaviour |
|---|---|---|
| `CorsairK65MiniControllerDetect.cpp` | `table::identify` | VID 0x1B1C, PID 0x1BAF, interface 1 only |
| `CorsairK65MiniController::LightingControl` | `report::init_packets` | Three 1025-byte init packets (`08 01 03 ..05 02`, `08 02 6E`, `08 0D 01 22`) |
| `RGBController_CorsairK65Mini::SetupZones` | `lib.rs::initialize` | 64 LEDs; matrix 5x15; `led_positions` array |
| `CorsairK65MiniController::SetLEDs` | `report::frame` | 1025-byte report `08 06 01 73 01 00 00 12 00`, sparse color writes at `10 + position*3` |
| `RGBController_CorsairK65Mini::KeepaliveThread` | omitted | no thread in module API; frame-driven only |

Changes: Direct mode only, no hardware effects. Keepalive (50 s, 3 s sleep) omitted. Shutdown no off packet. Serial reads omitted. Test status: simulated only.

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: directory level (no named file resolved): all source files of the upstream controller directories.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/CorsairPeripheralController/CorsairK55RGBPROXTController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairK55RGBPROXTController.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/CorsairK55RGBPROXTController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairK55RGBPROXTController.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/CorsairK65MiniController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairK65MiniController.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/CorsairK65MiniController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairK65MiniController.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/CorsairPeripheralController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairPeripheralController.cpp) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 09 Jan 2020 |
| `Controllers/CorsairPeripheralController/CorsairPeripheralController.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairPeripheralController.h) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 09 Jan 2020 |
| `Controllers/CorsairPeripheralController/CorsairPeripheralControllerDetect.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/CorsairPeripheralControllerDetect.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairK55RGBPROXT.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairK55RGBPROXT.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairK55RGBPROXT.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairK55RGBPROXT.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairK65Mini.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairK65Mini.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairK65Mini.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairK65Mini.h) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairPeripheral.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairPeripheral.cpp) | no SPDX line in header | no copyright/author line in header |
| `Controllers/CorsairPeripheralController/RGBController_CorsairPeripheral.h` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/CorsairPeripheralController/RGBController_CorsairPeripheral.h) | GPL-2.0-or-later | Adam Honse (CalcProgrammer1) 09 Jan 2020 |

Changes: ported to a Rust freeRGB module (process module under `modules/corsair-k65-mini/`); behaviour changes are listed in the hand-written sections above.

Names in the hand-written record that could not be resolved in the checkout (manual check needed): `CorsairK65MiniControllerDetect.cpp`

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
