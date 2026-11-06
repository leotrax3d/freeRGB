// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream: AsusAuraMousematController.cpp UpdateLeds: 65-byte report,
/// `EE C0 81 00 00` then 15 groups `[00 R G B]` (60 bytes).
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; 65];
    p[0] = 0xee;
    p[1] = 0xc0;
    p[2] = 0x81;
    // p[3], p[4] = 0
    for (i, c) in colors.iter().enumerate().take(15) {
        let b = 5 + i * 4;
        p[b] = 0x00;
        p[b + 1] = c.r;
        p[b + 2] = c.g;
        p[b + 3] = c.b;
    }
    p
}
