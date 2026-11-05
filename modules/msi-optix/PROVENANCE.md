# Provenance: freeRGB module msi-optix

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit
bc77470a7cba56af60989e54ba910d5ac66a3ac, Controllers/MSIOptixController.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| MSIOptixControllerDetect.cpp | 	able::identify | VID 0x1462, PID 0x3FA4, interface 0, usage page 0xFF00, usage 1 |
| MSIOptixController.cpp SetDirect | eport::direct | 168-byte feature report, report ID 0x72, header blocks, 12 RGB colors at offset 95, fixed brightness 0x64 |
| RGBController_MSIOptix.cpp constructor/SetupZones | lib.rs::initialize | 12-LED Backside zone, direct only |

Changes: Direct mode only; firmware/version reads omitted; effect modes omitted. Test status: simulated only.
