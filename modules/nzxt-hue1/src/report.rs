// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Byte layout constants from upstream `NZXTHue1Controller.cpp`.
pub const MAX_COLORS: usize = 40;

/// Upstream: NZXTHue1Controller.cpp :: Initialize (first write).
/// 65-byte `01 5C` report.
pub fn init() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0x01;
    p[1] = 0x5c;
    p
}

/// Upstream: NZXTHue1Controller.cpp :: Initialize (second write).
/// 65-byte `01 5D` report that starts the accessory reporting.
pub fn start_reporting() -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0x01;
    p[1] = 0x5d;
    p
}

/// Upstream: NZXTHue1Controller.cpp :: SendPacket with
/// `HUE_1_MODE_FIXED` (0x00) and the 8-bit fields of `RGBController_NZXTHue1.cpp
/// DeviceUpdateLEDs`; direction = false, speed = 0.
/// Two 65-byte writes: packet 1 header `02 4B <mode> <options> <idx<<5|speed>`
/// carries the first 20 colors in G/R/B order (3 bytes each) from byte 5,
/// packet 2 header `03` carries the remaining colors.
pub fn direct(colors: &[Rgb]) -> Vec<Vec<u8>> {
    let colors = &colors[..colors.len().min(MAX_COLORS)];
    // First packet: mode field, options, color index/speed, up to 20 colors.
    let mut p1 = vec![0; 65];
    p1[0] = 0x02;
    p1[1] = 0x4b;
    p1[2] = 0x00; // HUE_1_MODE_FIXED
    p1[3] = 0x00; // direction bit (1 << 4) not set
    p1[4] = 0x00; // (color_idx << 5) | speed
    let first = colors.len().min(20);
    for (i, c) in colors[..first].iter().enumerate() {
        let q = 5 + i * 3;
        // upstream fills G at +0, R at +1, B at +2
        p1[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    // Second packet: the remaining colors. Upstream copies them from the start
    // of `color_data` again (NZXTHue1Controller.cpp SendPacket passes
    // `color_data`, not `color_data + first`), so this reproduces that exact
    // behaviour byte for byte.
    let rest = colors.len() - first;
    let mut p2 = vec![0; 65];
    p2[0] = 0x03;
    for (i, c) in colors[..rest].iter().enumerate() {
        let q = 1 + i * 3;
        p2[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    vec![p1, p2]
}
