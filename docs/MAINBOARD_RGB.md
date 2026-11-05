# Motherboard RGB/ARGB (ASUS Aura USB, Gigabyte RGB Fusion 2 USB, ASRock Polychrome USB)

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
| ASUS Aura addressable controllers | `0b05:1867`, `0b05:1872`, `0b05:18a3`, `0b05:18a5` | any | Not driven by the core; the GPL module `modules/asus-aura-usb` (installed through device setup) can claim them |
| MSI Mystic Light USB | `1462:7b00`–`1462:7eff`, `1462:1720` | any | Identified only (board-specific 112/162/185/761-byte families not implemented) |
| ASRock Polychrome USB | `26ce:01a2`, `26ce:01a6` | any | Driven (per-LED; ARGB1/ARGB2/ARGB3 up to 100 LEDs each, 12 V and onboard zones as fixed LEDs) |
| SMBus/I²C motherboard RGB (older ASUS/MSI/Gigabyte/ASRock boards, ENE/Aura SMBus, Polychrome SMBus) | — | — | **Not supported** (no SMBus access in freeRGB) |

"Identified only" controllers appear in diagnostics and are never written to.

## Configuration (modern GUI: Devices page; Classic: Monitoring → Motherboard RGB; daemon RPC)

Supported controllers are detected and opened at startup like any other device.
`mainboard-rgb.json` (per-user config directory) holds:

- `enabled` — global "Manage motherboard RGB" switch, default **on**;
- per controller (`vvvv:pppp`) up to four headers with `leds` (0..512) and an
  optional `name` shown on the layout canvas. Presets for 8/12/16/18-LED fans
- ASUS Aura headers also take `order` (`rgb` default, `rbg`, `grb`, `gbr`, `brg`,
  `bgr`) and `fallback` (`{"mode":"spectrum"}` default, `{"mode":"off"}`,
  `{"mode":"static","color":"#rrggbb"}`). `order` is the byte order the strip
  expects: if a red test LED shows green and a green one shows red, choose `grb`.
  `fallback` is what the header shows when freeRGB stops: the firmware spectrum
  effect (the previous effect cannot be read back), the firmware "off" effect, or
  one last direct frame in the given colour. Both are volatile: nothing is
  written to flash. Gigabyte and ASRock headers ignore these two fields.
- `mainboard_rgb.get` also returns `detected` per controller: what the controller
  reports (ASUS: which of the four ARGB headers exist and the fixed-LED count; the
  protocol does not report attached strip lengths, so the LED count stays a user
  setting). The modern GUI can export/import the settings as JSON (validated; the
  per-header test pattern is an owner step, see below).

Owner acceptance (needs the real board, not run by the worker): set a header's
LED count, apply a red/green/blue frame, pick the `order` that shows the right
colours, then close freeRGB and check the chosen `fallback`.
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

## ASRock Polychrome USB wire facts

65-byte output reports (report ID 0), each answered by a 64-byte input report.
`14` header reads: config `02` = LED count per zone (RGB1, RGB2, ARGB1, ARGB2,
PCH, IO, PCB, ARGB3/audio; `0x1e` = absent), `03` = RGB-swap bitmask, `01` =
availability bitmask. `11 zone` reads a zone's effect (mode at 4, colour 5..7,
speed 8). Anything implausible stops before the first write.

Taking control: the LED-count table is written (`15 02 …`) only if the user's ARGB
counts differ (30 is rejected because it equals the absent marker), hardware
RGB swap is switched off (`15 03 00`, it flickers in per-LED mode) and colour
bytes are ordered per zone instead (GRB, or RGB where the original swap bit was
set). Frames: `10 _ ff e3` with LEDs from byte 9 (18 per report), then
`10 _ ff e4` from byte 5 (19 per report), byte 0x40 = `0x65`; zones packed in
order, padded to the sum of zone maxima + 8 LEDs. Release writes back the
original LED-count table, swap byte and each zone's effect (`10 zone mode r g b
speed ff`; a per-LED mode falls back to spectrum cycle). The commit/save command
`0x12` is never sent.

## GPL hardware modules

`modules/integration` (merged here) adds GPL modules, among them
`modules/asus-aura-usb`. The core adapters above take precedence: the hardware
scan never lets a module claim an interface a built-in adapter drives. Modules
are installed per device through device setup (signed catalog), not by
default. The GPL Gigabyte module branch (`module/gigabyte-rgbfusion2`) was not
taken: it is a stub (no controller query, no effect switch, wrong offset field,
no fallback); the core `gigabyte_fusion2.rs` is used instead.

## OpenRGB reference

Protocol facts were checked against OpenRGB commit
`c43424b4ba94748b913a6551c274251a5211510c` (GPL-2.0) as a reference only; no code
was copied. Files: `Controllers/AsusAuraUSBController/AsusAuraUSBControllerDetect.cpp`,
`Controllers/GigabyteRGBFusion2USBController/GigabyteRGBFusion2USBController{.cpp,.h}`,
`.../GigabyteRGBFusion2USBControllerDetect.cpp`, `.../GigabyteFusion2USB_Devices.h`,
`Controllers/ASRockPolychromeUSBController/ASRockPolychromeUSBController{.cpp,.h}`,
`.../RGBController_ASRockPolychromeUSB.cpp`, `.../ASRockPolychromeUSBControllerDetect.cpp`
(unchanged at commit `7182807a288a9e8107a84fbebc44239677f1ea24`, which lists the same ASUS/Gigabyte IDs),
`Controllers/MSIMotherboardController/MSIMotherboardControllerDetect.cpp`.

## Hardware validation

No motherboard RGB controller has been tested on real hardware by the maintainers
for this release; all of the above is verified against mock transports only
(`asus_aura.rs`, `gigabyte_fusion2.rs`, `asrock_polychrome.rs` tests). Please report results with the
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
