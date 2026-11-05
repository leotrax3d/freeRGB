# Provenance: freeRGB module `corsair-k55-pro-xt`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/CorsairPeripheralController`
(see `CorsairK55RGBPROXTController`).

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `CorsairK55RGBPROXTController.cpp` `LightingControl` | `report::init_packets` | 65-byte software lighting writes (`08 01 03 .. 02`, `08 02 5F`, `08 0D 00 01`) |
| `CorsairK55RGBPROXTController.cpp` `SetLEDs` | `report::frames` | key-mapped RGB banks R/G/B (137 bytes each; 419-byte stream flattened), first packet `00 08 06 00 9B 01 …` +53 bytes, then six `00 08 07 00 …61`-byte continuations |
| `RGBController_CorsairK55RGBPROXT.cpp` `SetupZones` | `lib.rs::initialize` | 111 LEDs, single "Keyboard" zone, 24×6 matrix |
| `DetectCorsairK55RGBPROXTControllers` | `table::identify` | VID 0x1B1C, PID 0x1BA1, usage page 0xFF42 |

Changes: Direct mode only; hardware modes, keepalive thread and Mode-switch omitted. `keys[]` imported via generator (`keys.rs`). Tests: simulated only.