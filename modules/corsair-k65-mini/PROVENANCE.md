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