# Provenance: freeRGB module `razer-chroma`

This module is a Rust port of the Razer controller driver from **OpenRGB**.
It is a derivative work and is distributed under the **GNU General Public
License, version 2 or (at your option) any later version** (`GPL-2.0-or-later`),
the license of every upstream file used. The full license text is in
`COPYING`. The rest of freeRGB remains `MIT OR Apache-2.0`; the module is a
separate program that talks to the freeRGB service only through the generic,
documented module protocol (`docs/HARDWARE_MODULES.md`). No SignalRGB code is
used.

## Upstream

| | |
|---|---|
| Project | OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB |
| Commit | `bbc77470a7cba56af60989e54ba910d5ac66a3ac` (2026-10-21) |
| Copyright | Adam Honse (CalcProgrammer1) and the OpenRGB contributors |
| License of used files | `SPDX-License-Identifier: GPL-2.0-or-later` in each file header (checked per file, not only the repository license) |

## Files and what was taken

| Upstream file | Taken into | Behaviour preserved |
|---|---|---|
| `Controllers/RazerController/RazerDevices.cpp`, `RazerDevices.h` | `src/devices_table.rs` (generated) | Device table: name, PID, device type, matrix type, transaction ID, LED ID, rows/cols, zones (name, type, size), keyboard-layout flag; PID constants |
| `Controllers/RazerController/RazerControllerDetect.cpp` | `src/devices_table.rs` (generated), `src/table.rs` | Detector registrations (VID, PID, interface, usage page, usage, detector function); matching rule "every given criterion must match" |
| `Controllers/RazerController/RazerController/RazerController.h` | `src/report.rs`, `src/table.rs` | 91-byte `razer_report` layout (report ID, status, transaction ID, remaining packets, protocol type, data size, class, command, 80 argument bytes, CRC, reserved), status/LED/matrix/storage constants |
| `Controllers/RazerController/RazerController/RazerController.cpp` | `src/report.rs`, `src/lib.rs`, `src/table.rs` | `razer_calculate_crc` (XOR of bytes 3..88), `razer_create_report`, brightness (standard/extended), custom frame (standard/extended/linear), custom mode (standard/extended), device mode, get firmware; `SetLEDs` (one report per row, 1 ms before each row and before the mode report); `razer_set_brightness`; `razer_get_firmware` timing (2 ms / 5 ms); constructor special cases: report index 0x07 for Leviathan V2/V2X, software device mode for Charging Pad Chroma, Chroma Mug, Firefly HyperFlux; `GetMaxBrightness` (100 for DeathAdder Essential V2); response matching from `razer_usb_query` |
| `Controllers/RazerController/RazerController/RGBController_Razer.cpp` | `src/lib.rs` | Direct mode: brightness set to the device maximum when Direct is selected; LED naming from `SetupZones` (zone name, plus " LED n" for zones with more than one LED); LED order (zones in table order, rows then columns) |
| `Controllers/RazerController/RazerDeviceGuard.cpp` | `src/guard.rs` | Machine-wide mutex `Global\RazerLinkReadWriteGuardMutex` held around each report write (coexistence with Razer software) |

## Changes made for freeRGB (modifications per GPL section 2a)

* Translated from C++ to Rust. The device and detector tables are generated
  by `scripts/openrgb/port-razer-devices.mjs`; a re-run against the pinned
  commit must reproduce `src/devices_table.rs` exactly.
* Transport: reports go through the freeRGB host (`send_feature`/`get_feature`
  module calls) instead of hidapi. The host owns the device handle.
* Only the **Direct** mode is offered (freeRGB renders all effects itself).
  Hardware effect modes, the firmware effect-list query (0x0F:0x80/0x81),
  keyboard layout and variant queries and the serial number query are not
  performed. The serial number is intentionally never read.
* LED names in multi-row zones are row-qualified (`"<zone> LED r.c"`) because
  freeRGB requires unique LED names; upstream repeats the column name per row.
* Shutdown: like upstream nothing is sent (the custom frame is not saved to
  the device, so the firmware's stored effect returns after a power cycle). If
  the service asks for a final colour, one Direct frame in that colour is sent.
* The keep-alive thread for Blade 14 (2021/2022) is not ported: freeRGB sends
  frames continuously while it runs, which upstream notes is sufficient to
  keep the lighting active. Not verified on that hardware.
* An abandoned Razer guard mutex is treated as acquired (upstream releases and
  waits again; both end with this process owning the mutex).

## Not ported (reported as "not yet supported", never guessed)

* Devices with `RAZER_MATRIX_TYPE_CUSTOM` (per-device LED effect reports),
  `RAZER_MATRIX_TYPE_EXTENDED_ARGB` (Chroma Addressable RGB Controller: second
  interface with 320-byte reports) and devices with a keyboard layout map
  (`KeyboardLayoutManager`).
* Detectors other than `DetectRazerControllers` (Kraken, Kraken V3/V4, Hanbo,
  ARGB controller) — separate protocols, candidates for their own modules.
* Devices paired behind a wireless receiver/dock (`GetPairedDevicePIDs`,
  `SetWaitForResponse`).

`freergb-module-razer-chroma --list-devices` prints exactly the supported set
(125 of 205 upstream detector entries at the pinned commit).

## Verification status

| Level | Scope |
|---|---|
| Ported | All 125 listed device entries (table data + the code paths above). |
| Simulated | Byte-exact report construction and CRC (`report.rs` tests); full initialize/frame/shutdown sequence for the Razer Cobra against a simulated device; every supported layout validated; detector matching incl. wrong interface/usage. |
| Confirmed on hardware | See `docs/HARDWARE_MODULES.md`, section "Hardware validation log". |

## Corresponding source

Every binary package of this module is published together with a source
archive (`freergb-module-razer-chroma-<version>-source.zip`) containing this
directory, the shared crates it builds against (`freergb-core`,
`freergb-module-api`), the workspace `Cargo.toml`/`Cargo.lock`, the generator
script and the license files — everything needed to rebuild the exact binary
with `cargo build --release -p freergb-module-razer-chroma`. The catalog lists
that archive with its SHA-256 next to each binary package.
