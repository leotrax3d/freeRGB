# Provenance

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB,
Commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
License of every used upstream file: GPL-2.0-or-later (SPDX headers verified across
`Controllers/LenovoControllers`).

The branch version's detector/device tables were audited as faithful (24 rows,
VID/PID/names verified against the detect files); the port status ledger records
which subset is actually claimed.

## Behavior transfer (audited sub-controllers)

| Upstream file/function | Rust file/function | Behavior transferred |
|---|---|---|
| `LenovoUSBController.cpp :: setDeviceSoftwareMode` | `report.rs :: legacy_software_mode` | 192-byte feature `07 B2` |
| `LenovoUSBController.cpp :: sendLedsDirect` | `report.rs :: legacy` | 192-byte feature packets `07 A0+zone`, 47 LEDs per packet with (id, R, G, B) entries, per-zone LED id groups from `legacy_ids.rs` (Y740/Y760/Y750 verified against LenovoDevices.h) |
| `LenovoUSBController_Gen7_8.cpp :: setLedsDirectOn` / `setLedsDirect` | `report.rs :: gen7_direct_on/gen7` | 960-byte features `07 D0 C0 03 .. <profile>` (Gen10 length header) and `07 A1 C0 03 (lo hi R G B)*` |
| `Lenovo4ZoneUSBController.cpp :: UpdateLedData` | `report.rs :: four_zone` | 33-byte feature `CC 16`, static effect, 4 zone RGB at 5..16 |
| `LenovoM300Controller.cpp :: SetMode` (Static) | `report.rs :: m300` | 65-byte write `25`, static mode 3, checksum fold-sum at 0x40 |
| `LenovoK500Controller.cpp :: SetMode` (Customize) | `report.rs :: k500` | 9-byte setup feature `14 00 5A`, 65-byte write with per-key (byte,bit) bits from `k500_matrix.rs` (104 entries verified) |

## Supported subset

`supported = true` only for: seven 4-zone rows (0xC963/0xC955/0xC973/0xC975/0xC984/0xC985/0xC965),
k500 0x60D5, m300 0x60E4, legacy ANSI rows 0xC935/0xC957/0xC967 and gen7 0xC977
(14 of 24 detector rows).

## Nicht unterstützt (audit-Gründe)

* k510 0x619A — `k510()` sends no packets (upstream SetMode writes a 64-byte frame
  with mode/brightness/speed/direction + colors, not ported).
* Gen10 0xC195/0xC197 — packet header must carry the payload length in bytes 2/3
  and the D0 direct-on packet must be re-asserted before every update; 0xC197 is
  also missing the 0xDD logo LED (119 vs 120).
* Gen7 laptops 0xC978/0xC987/0xC988/0xC997/0xC998 — zone-ID high bytes
  (`zone_id << 8 | led_num`) are zeroed, so logo/vents/neon colors land on the
  wrong device zones.
* Y750 0xC956 — table LED count 145 makes the Y750 additional LEDs (0xAA/0xAD/0xAC)
  unreachable; upstream variant queries not ported.
* K500 bytes 0x14..0x2D of the color packet carry the upstream default palette;
  the module sends zeros. Documented deviation.

## Changes

* Direct mode only; hardware effect modes, brightness/speed controls, keepalive
  threads and variant information queries (getInformation) not exposed by the module API.

## Test status

Simuliert geprüft, nicht auf Hardware bestätigt.

<!-- BEGIN GENERATED PROVENANCE (scripts/fill-module-provenance.py) -->

## Upstream attribution (generated)

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`.
Licence of the used files: GPL-2.0-or-later (SPDX column below is read from each header; where a header says otherwise the module licence needs a manual check).
Scope: files named in the hand-written record.

| Upstream file | URL at commit | SPDX | Copyright / author lines in header |
|---|---|---|---|
| `Controllers/LenovoControllers/LenovoUSBController/LenovoUSBController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/LenovoControllers/LenovoUSBController/LenovoUSBController.cpp) | GPL-2.0-or-later | Cooper Hall (geobot19) 17 Apr 2022 |
| `Controllers/LenovoControllers/LenovoUSBController_Gen7_8/LenovoUSBController_Gen7_8.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/LenovoControllers/LenovoUSBController_Gen7_8/LenovoUSBController_Gen7_8.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/LenovoControllers/Lenovo4ZoneUSBController/Lenovo4ZoneUSBController.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/LenovoControllers/Lenovo4ZoneUSBController/Lenovo4ZoneUSBController.cpp) | GPL-2.0-or-later | no copyright/author line in header |
| `Controllers/LenovoControllers/LenovoM300Controller/LenovoM300Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/LenovoControllers/LenovoM300Controller/LenovoM300Controller.cpp) | GPL-2.0-or-later | Wayne Riordan 09 Jan 2024 |
| `Controllers/LenovoControllers/LenovoLegionK500Controller/LenovoK500Controller.cpp` | [bbc77470a7cb](https://gitlab.com/CalcProgrammer1/OpenRGB/-/blob/bbc77470a7cba56af60989e54ba910d5ac66a3ac/Controllers/LenovoControllers/LenovoLegionK500Controller/LenovoK500Controller.cpp) | GPL-2.0-or-later | MX1D 10 Aug 2026 |

Changes: ported to a Rust freeRGB module (process module under `modules/lenovo/`); device/detector tables generated by `scripts/openrgb/port-lenovo-devices.mjs`; behaviour changes are listed in the hand-written sections above.

Generated by mechanical header extraction; not a legal review.
<!-- END GENERATED PROVENANCE -->
