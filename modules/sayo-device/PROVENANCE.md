# Provenance: freeRGB module `sayo-device`

SPDX-License-Identifier: GPL-2.0-or-later

Upstream: OpenRGB, https://gitlab.com/CalcProgrammer1/OpenRGB, commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`, `Controllers/SayoDeviceController`.

| Upstream function | Rust function | Adopted behaviour |
|---|---|---|
| `SayoDeviceControllerDetect.cpp` | `table::identify` | VID 0x8089, PID 0x0007, usage page 0xFF11, usage 0x0002 |
| `SayoDeviceController::SendPacket` | `report::direct` | 64-byte `hid_write`: 0x21, 0x12, 16-bit LE checksum, 27-byte payload (with RGB and static mode), zero-filled |
| `RGBController_SayoDevice::DeviceUpdateLEDs` | `lib.rs::frame` | single feature write for one LED |

Changes: Direct mode only. Test status: simulated only.