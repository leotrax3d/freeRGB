# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/MSIMotherboardController/`. The detector inventory is generated
by `scripts/openrgb/port-msi-devices.mjs` from
`MSIMotherboardControllerDetect.cpp`.

The freeRGB port uses brokered HID feature reports and never reads a serial
number or a device path. It preserves the upstream 0x52 variants and the
0x50/0x51 761-byte configuration/direct protocol. Firmware effects are not
selected by the freeRGB frame API; frames are sent as direct RGB state.

## Audit correction

This description overstated the port. OpenRGB has four distinct HID
sub-controllers (`MSIMotherboard112Controller`, `162Controller`,
`185Controller`, and `761Controller`), each with initialization, mode setup,
zone mapping, direct update, and cleanup code. The Rust implementation does
not preserve that complete lifecycle, so all 124 generated detector rows (123
unique IDs) are **nicht portiert** and `identify()` returns `supported=false`.

| Upstream path | Devices | Rust mapping | Status |
|---|---:|---|---|
| `MSIMotherboard112Controller` | detector rows mapped to 112-byte path | detector table only | nicht portiert |
| `MSIMotherboard162Controller` | detector rows mapped to 162-byte path | detector table only | nicht portiert |
| `MSIMotherboard185Controller` | detector rows mapped to 185-byte path | detector table only | nicht portiert |
| `MSIMotherboard761Controller` | detector rows mapped to 761-byte path | detector table only | nicht portiert |

Every individual device is the generated row in `src/devices_table.rs`; none
is supported until its complete upstream path has byte-exact tests.
