# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/AsusAuraUSBController/`. Detector data is generated from the
pinned detector source. The built-in ASUS Aura USB motherboard device
`0b05:19af` is deliberately omitted because freeRGB owns it in-tree.

The module uses brokered HID only, with no serial or path reads. The direct
0xEC/0x40 65-byte packet framing and 20-LED chunking are preserved for the
common Aura USB transport; controller-specific firmware effects remain outside
the frame API.

## Audit correction

The former claim treated a common-looking packet as a complete family port.
OpenRGB dispatches these 100 non-built-in detector rows to multiple
controller-specific paths. Their complete initialization, LED mapping, direct
mode and cleanup are not represented in Rust. All 100 rows are **nicht
portiert**; the built-in 0b05:19af row remains excluded, and `identify()` now
returns `supported=false` for every inventoried external row.

| Upstream path | Devices | Rust mapping | Status |
|---|---:|---|---|
| Aura USB controller-specific paths | 100 external rows | generated detector table only | nicht portiert |
