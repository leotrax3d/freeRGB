# Provenance

Ported from OpenRGB commit `bbc77470a7cba56af60989e54ba910d5ac66a3ac`,
`Controllers/AsusAuraUSBController/`. Detector data is generated from the
pinned detector source. The built-in ASUS Aura USB motherboard device
`0b05:19af` is deliberately omitted because freeRGB owns it in-tree.

The module uses brokered HID only, with no serial or path reads. The direct
0xEC/0x40 65-byte packet framing and 20-LED chunking are preserved for the
common Aura USB transport; controller-specific firmware effects remain outside
the frame API.
