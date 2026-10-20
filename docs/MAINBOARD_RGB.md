# Motherboard RGB/ARGB: experimental ASUS USB backend

This controls lighting, not fan power, PWM/DC, pump duty or temperatures. It needs
the existing OS HID driver, not PawnIO. Mainboard SuperIO fans use a different
provider. Broad manufacturer coverage remains work in progress.

Initial allowlist: ASUS `0b05:19af`, vendor interface usage page `ff72`, usage `00a1`.
Other ASUS IDs and MSI/Gigabyte/ASRock protocols are not enabled by inference.
Detection is experimental; normal startup leaves this adapter unopened.

Advanced opt-in, through the existing launcher:

```powershell
freergb-launcher --enable-asus-aura --asus-argb-leds 24,24,0,0
```

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
