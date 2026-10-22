# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSIMotherboardController/`. The detector inventory is generated
by `scripts/openrgb/port-msi-devices.mjs` from
`MSIMotherboardControllerDetect.cpp`.

The freeRGB port uses brokered HID feature reports and never reads a serial
number or a device path. It preserves the upstream 0x52 variants and the
0x50/0x51 761-byte configuration/direct protocol. Firmware effects are not
selected by the freeRGB frame API; frames are sent as direct RGB state.
