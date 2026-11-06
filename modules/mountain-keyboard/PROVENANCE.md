# Provenance: freeRGB module `mountain-keyboard`

SPDX-License-Identifier: GPL-2.0-or-later

The freeRGB core and module API (MIT OR Apache-2.0) contain no GPL code; no
SignalRGB code is used. Upstream code is GPL-2.0-or-later and was ported read-only.

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
`bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/MountainKeyboardController`,
including `MountainKeyboardController.cpp/.h`, `Mountain60KeyboardController.cpp/.h`,
`RGBController_MountainKeyboard.cpp/.h` and `RGBController_Mountain60Keyboard.cpp/.h`.

| Upstream file | Rust file | Adopted behaviour |
|---|---|---|
| `MountainKeyboardControllerDetect.cpp` | `devices_table.rs` | VID 0x3282, three rows (Everest, Everest 60 ANSI/ISO): interface 3 for Everest; interface 2, usage page 0xFFFF for Everest 60 |
| Other controller files | (not ported) | Chunked 65-byte direct-colour packets, wheel config, effect modes, save/select remain unported |

Sub-controllers:
- `MountainKeyboardController` (Mountain Everest): not ported
- `Mountain60KeyboardController` (Mountain Everest 60): not ported

Changes from upstream: brokered HID; no serial/product reads; no packet bytes are
fabricated or sent; `identify()` reports supported=false for every row.

Test status: simulated only; no hardware confirmed. Three-row inventory test and
unsupported-initialize tests.