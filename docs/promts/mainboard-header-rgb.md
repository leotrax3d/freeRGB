# Plan — RGB/ARGB devices on motherboard headers (fans, strips, coolers)

OpenRGB drives devices on the motherboard's 12 V RGB and 5 V ARGB headers by
talking to the **motherboard's RGB controller**, not to the fan itself. The fan
or strip has no data link of its own; the controller sets colour per header
(12 V RGB: one colour per header) or per LED (ARGB: one address per LED, LED
count set by the user). freeRGB already follows the same model
([MAINBOARD_RGB.md](../MAINBOARD_RGB.md)).

## Current state on `main`

| Controller | Transport | State |
|---|---|---|
| ASUS Aura USB | HID | driven (simulated), per-header colour order, fallback, header detection |
| Gigabyte RGB Fusion 2 USB (IT5702/IT8297) | HID | driven (D_LED1/D_LED2) |
| Gigabyte Fusion 2 variants (5711/8950/a100) | HID | identified only |
| ASRock Polychrome USB | HID | driven |
| MSI Mystic Light (module `msi-motherboard`) | HID | 112/162-byte families ported; 185 blocked (serial, 13 ms timing); 761 blocked (needs DMI board name) |
| ENE/Aura SMBus, ASRock/Gigabyte SMBus, older boards | SMBus | not supported (no native SMBus access) |
| Corsair Lighting Node, Commander, NZXT Hue, Lian Li hubs | HID | modules exist (USB hubs, not headers, but same use case) |

Nothing on headers is hardware-accepted yet; owner tests go to
[07-hardware-acceptance.md](fancontrol/07-hardware-acceptance.md).

## Plan

**P1 — Unblock MSI Mystic Light 185/761 (software, now possible).**
A1 timers, A2 serial reads and A3 DMI board identity are on `main`. Port the
185-byte family (serial → real PID, 13 ms per-LED pacing via host timer) and
the 761-byte family (board name from A3). Owner: module porter (friend), the
API is ready. MSI is the largest remaining HID header controller.

**P2 — Gigabyte Fusion 2 variants (software).**
Extend the core driver for `048d:5711`, `048d:8950`, `0414:a100`: header count,
calibration table, byte-exact tests from OpenRGB `RGBFusion2USBController`.
Core code (`crates/`), done here.

**P3 — Native SMBus/I²C access (core, needs owner decision).**
Covers ENE/Aura SMBus boards, ASRock/Gigabyte SMBus boards and also ~900
RAM/GPU detectors. Windows needs a kernel-level port driver (PawnIO is the
current OpenRGB choice; WinRing0 is unsafe). Linux uses `i2c-dev`.
Steps: (a) owner decision on the driver and its licence/signing, (b) broker
the bus through the existing protected helper (`MAINBOARD_BROKER.md`) with an
address/register allow-list per module, (c) replace `SimulatedBus` by a native
backend behind the same API 2, (d) keep the simulated tests. Needs UAC for the
driver install and hardware for acceptance.

**P4 — User-facing header setup (software).**
One "Headers" page in both GUIs for all controllers: per header type
(RGB/ARGB), LED count, colour order, device name ("Front fans"), test pattern,
and mapping into the shared canvas so effects run across headers, fans and
peripherals. Reuse the ASUS header config model for Gigabyte/ASRock/MSI.
Optional link with fan control: show a header's fan next to its cooling
channel (name only; no automatic coupling).

**P5 — Conflict handling.**
Detect running vendor software (Armoury Crate, MSI Center, RGB Fusion,
Polychrome) and warn before taking the controller; already partly documented
in MAINBOARD_RGB.md.

**Order:** P1 (friend) and P2 + P4 (here, in parallel, disjoint files) now;
P5 small, with P4; P3 only after the owner decides on the driver.
