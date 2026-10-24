# Motherboard RGB/ARGB (ASUS Aura USB, Gigabyte RGB Fusion 2 USB)

This controls lighting, not fan power, PWM/DC, pump duty or temperatures. It needs
the existing OS HID driver, not PawnIO. Mainboard SuperIO fans use a different
provider.

## Supported controllers

See [MAINBOARD_RGB_DEVICES.md](MAINBOARD_RGB_DEVICES.md) for the list of known USB controllers.


| Family | USB IDs | Interface | State |
| --- | --- | --- | --- |
| ASUS Aura motherboard USB | `0b05:18f3`, `0b05:1939`, `0b05:19af`, `0b05:1aa6`, `0b05:1bed` | `ff72`/`00a1` | Driven (fixed RGB, 12 V headers, up to 4 ARGB headers, Gen1 ARGB selection) |
| Gigabyte RGB Fusion 2 USB (IT5702/IT8297) | `048d:5702`, `048d:8297` | `ff89`/`00cc` | Driven (D_LED1/D_LED2 ARGB headers) |
| Gigabyte Fusion 2 variants | `048d:5711`, `048d:8950`, `0414:a100` | `ff89`/`00cc` | Identified only (more headers/other calibration) |
| ASUS Aura addressable controllers | `0b05:1867`, `0b05:1872`, `0b05:18a3`, `0b05:18a5` | any | Identified only |
| MSI Mystic Light USB | `1462:7b00`–`1462:7eff`, `1462:1720` | any | Identified only (board-specific 112/162/185/761-byte families not implemented) |
| ASRock Polychrome USB | `26ce:01a2` | any | Identified only |
| SMBus/I²C motherboard RGB (older ASUS/MSI/Gigabyte/ASRock boards, ENE/Aura SMBus, Polychrome SMBus) | — | — | **Not supported** (no SMBus access in freeRGB) |

"Identified only" controllers appear in diagnostics and are never written to.

## Configuration (modern GUI: Devices page; Classic: Monitoring → Motherboard RGB; daemon RPC)

Supported controllers are detected and opened at startup like any other device.
`mainboard-rgb.json` (per-user config directory) holds:

- `enabled` — global "Manage motherboard RGB" switch, default **on**;
- per controller (`vvvv:pppp`) up to four headers with `leds` (0..512) and an
  optional `name` shown on the layout canvas. Presets for 8/12/16/18-LED fans
  (times chained fans) are offered by the GUI.

RPC: `mainboard_rgb.get`, `mainboard_rgb.save {settings}`; saving releases the
controllers so the next scan reopens them with the new header table. A header
with 0 LEDs is left out of frames and keeps its firmware effect.

If ASUS Armoury Crate / Aura Lighting Service, MSI Center / Mystic Light,
Gigabyte RGB Fusion / Control Center or ASRock Polychrome RGB is running, freeRGB
does not open motherboard controllers and the Devices page shows a warning.
Other devices keep working.

Test override (unchanged flags, now optional):

```powershell
freergb-launcher --enable-asus-aura --asus-argb-leds 24,24,0,0
```

`--enable-asus-aura` opens ASUS Aura even when management is off or vendor
software runs, with the given LED counts instead of the saved settings.

## Gigabyte RGB Fusion 2 USB wire facts

64-byte feature reports, report ID `0xcc`. `cc 60` then a feature read returns the
controller information (product string at 12..40, strip-length classes at 8..10,
colour order words for D_LED1/D_LED2 at 44 and 48: bytes 2/1/0 are the index of
R/G/B within a triple; anything that is not a permutation is rejected before any
mode write). `cc 34 a b c` sets strip-length classes (32/64/256/512/1024 LEDs, two
nibbles per byte) and is only sent when D_LED1/D_LED2 change. `cc 32 mask` disables
the built-in effect of the selected headers (bit 0 D_LED1, bit 1 D_LED2); `cc 32 00`
re-enables the stored firmware effect on release/fault (volatile). Direct colour:
`cc 58|59 offLo offHi count` + up to 19 RGB triples, byte offset little endian.
Never sent: flash save, effect programming, fan or power commands.

## OpenRGB reference

Protocol facts were checked against OpenRGB commit
`c43424b4ba94748b913a6551c274251a5211510c` (GPL-2.0) as a reference only; no code
was copied. Files: `Controllers/AsusAuraUSBController/AsusAuraUSBControllerDetect.cpp`,
`Controllers/GigabyteRGBFusion2USBController/GigabyteRGBFusion2USBController{.cpp,.h}`,
`.../GigabyteRGBFusion2USBControllerDetect.cpp`, `.../GigabyteFusion2USB_Devices.h`,
`Controllers/ASRockPolychromeUSBController/*`,
`Controllers/MSIMotherboardController/MSIMotherboardControllerDetect.cpp`.

## Hardware validation

No motherboard RGB controller has been tested on real hardware by the maintainers
for this release; all of the above is verified against mock transports only
(`asus_aura.rs`, `gigabyte_fusion2.rs` tests). Please report results with the
"Hardware report" issue template.

## Earlier notes (ASUS Aura)

The option only affects a newly started daemon. Stop an existing daemon explicitly
before changing its startup options; closing the GUI does not stop it. Demo mode
rejects these options. The daemon also supports the same flags for backend testing;
`--no-hardware` never opens this controller. Header counts require the enable flag,
exactly four comma-separated integers in 0..512, default all zero. Zero excludes the
header from direct frames; it is not evidence that no LEDs are attached. Analog
12 V RGB headers each expose a single color slot, while a 5 V ARGB chain has manual
per-LED length. Check the board/device manuals; the electrical connectors differ.

The firmware config table supplies fixed RGB slots and numbers of RGB/ARGB headers,
not fan/strip models or reliable attached LED counts. We accept at most 16 fixed
slots and four ARGB headers. Unsupported tables, missing headers with nonzero user
counts, malformed replies and unsupported report sizes fail before RGB mode writes.
Configuration replies are 64..65 bytes, report marker `ec`, response `30`, with
60-byte table at offset 4. Table offsets `1b`, `1d`, `02` describe fixed slots,
analog headers and addressable headers. Names reflect firmware slot order; physical
connector mapping still needs confirmation.

The worker queries configuration (`ec b0`), then uses volatile direct mode
(`ec 35`, effect `ff`). Direct RGB payloads (`ec 40`) carry 20 RGB triplets per
65-byte output report; final packet applies the channel. ARGB offsets above 255
use the ninth-offset flag. Mainboard/fixed RGB uses direct channel 4; ARGB uses
channels 0..3. Any selected ARGB zone also selects controller-wide **Gen1** via
`ec 52 53 00 01`. This can affect Gen2 detection on other headers, including ones
excluded from frames; it is not isolated to the selected strip. The option takes
ownership of controller lighting and must not run concurrently with another RGB
controller application.

Disconnect/drop attempts firmware spectrum-cycle effect 4 for acquired effect
channels. The previous effect and previous Gen1/Gen2 state cannot currently be
read/restored. This is a known fallback, not restoration of prior user settings.
Failed fallback retains ownership for retry, preventing reacquisition until it
succeeds. No flash/persistent commit (`3f`), shutdown-effect update, fan or pump
commands are implemented. Gen1 selection lifetime and actual fallback behavior
still need hardware validation; use the vendor software to reconfigure afterward
if necessary. This adapter is not automatically enabled by prerequisite installation.

All effects, brightness and canvas regions use the existing daemon API and device
worker, shared by both GUIs. Existing API types are unchanged. The worker publishes
the discovered named layout before rendering; empty pre-discovery frames are
skipped. No protocol logic or extra dependencies are added to either GUI.

## Evidence and acceptance

Independent Rust implementation from documented wire facts; no upstream C++ code
is incorporated. Source reference pinned to OpenRGB commit
`7182807a288a9e8107a84fbebc44239677f1ea24`:
[base protocol](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBController/AsusAuraUSBController.cpp),
[mainboard protocol](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBController/AsusAuraMainboardController.cpp),
[detection](https://github.com/CalcProgrammer1/OpenRGB/blob/7182807a288a9e8107a84fbebc44239677f1ea24/Controllers/AsusAuraUSBController/AsusAuraUSBControllerDetect.cpp).

Mock transport tests cover malformed tables/no mode writes, packet chunking and
offset overflow, names/selected headers, frame-size rejection, only allowed RGB
commands, fallback and restoration-failure ownership. These are not hardware
acceptance. Real discovery, physical header correlation, Gen1/Gen2 behavior,
colors/per-LED order, fallback, unplug/replug and long-running tests remain open.
