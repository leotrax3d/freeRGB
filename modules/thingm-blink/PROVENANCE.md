# Provenance: freeRGB module `thingm-blink`

SPDX-License-Identifier: GPL-2.0-or-later

GPL-2.0-or-later Rust port of OpenRGB's ThingM blink(1) mk2 HID controller.
The MIT OR Apache-2.0 freeRGB core has no GPL code copied into it; no SignalRGB
code is used.

Upstream: OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.

| Upstream file | Ported behaviour |
|---|---|
| `ThingMControllerDetect.cpp` | VID/PID, usage page `FF00`, usage `01` matching |
| `BlinkController.h/.cpp` | 9-byte feature report, command `0x63`, RGB offsets, LED IDs 1/2 |
| `RGBController_BlinkController.cpp` | Fixed two-LED layout and LED naming |

Changes: brokered `send_feature` replaces hidapi; serial/path/product reads are
omitted; Direct is the only exposed mode. Off and Fade are not ported by policy.
Shutdown sends no packet, matching OpenRGB's no-save behaviour.

Status: ported and simulation-tested; not confirmed on hardware.
