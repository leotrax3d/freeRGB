# Provenance: freeRGB module `redragon-mouse`

SPDX-License-Identifier: GPL-2.0-or-later

This is a GPL-2.0-or-later Rust port of the OpenRGB Redragon mouse controller.
The freeRGB core remains MIT OR Apache-2.0 and contains no code from this
module. No SignalRGB code is used.

## Upstream

| Project | Commit | License |
|---|---|---|
| OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB | `bbc77470a7cba56af60989e54ba910d5ac66a3ac` | GPL-2.0-or-later |

| OpenRGB file | Port | Behaviour |
|---|---|---|
| `RedragonControllerDetect.cpp` | `src/devices_table.rs` | VID/PID/interface/usage-page registrations |
| `RedragonMouseController.h/.cpp` | `src/report.rs`, `src/lib.rs` | 16-byte feature reports, profile select, colour write and apply sequence |
| `RGBController_RedragonMouse.cpp` | `src/lib.rs` | One fixed Mouse LED layout; Direct-only policy |

## Changes and non-porting

Transport calls use freeRGB's brokered `send_feature`; no device path or serial
number is read. Effect modes, random breathing, profile persistence and other
OpenRGB UI modes are intentionally not exposed. Every detector not matching the
exact registered interface is returned as `supported: false` with a reason.

## Test status

Ported and simulation-tested: byte-exact builders, all 11 detector entries,
wrong-interface/usage rejection, layout validation, and initialize/frame/shutdown
sequence. Hardware confirmation has not been performed.
