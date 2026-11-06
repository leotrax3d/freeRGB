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
