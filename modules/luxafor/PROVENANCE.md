# Provenance: freeRGB module `luxafor`

SPDX-License-Identifier: GPL-2.0-or-later

GPL-2.0-or-later port of OpenRGB's Luxafor Flag HID controller. The freeRGB
core remains MIT OR Apache-2.0; no GPL code is copied into `crates/` and no
SignalRGB code is used.

| Upstream | Ported behaviour |
|---|---|
| `LuxaforControllerDetect.cpp` | VID `04D8`, PID `F372` detector |
| `LuxaforController.h/.cpp` | 9-byte Direct write packet, LED numbering 1–6, RGB byte order and timing field |
| `RGBController_Luxafor.cpp` | Flag/Rear zones and six-LED order |

Upstream commit: `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.

Changes: transport uses freeRGB brokered `write`; serial numbers and paths are
never read; only Direct is exposed. Fade, Strobe, Wave and Pattern modes are
not ported because this module's policy is Direct-only. Shutdown sends no
packet, matching OpenRGB.

Status: ported and simulation-tested; not confirmed on hardware.
