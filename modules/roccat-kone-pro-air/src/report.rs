// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `RoccatKoneProAirController.h`.
pub const REPORT_SIZE: usize = 16;

/// Upstream: RoccatKoneProAirController.cpp SendRGB(direct=true) writes 16 bytes:
/// `[00 10 10 0B 00 09 64 64 64 0B R G B R G B]`. Frame uses mode 0x09
/// (Direct), brightness 0x64 (100), speed filler 0x0B.
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; REPORT_SIZE];
    p[1] = 0x10;
    p[2] = 0x10;
    p[3] = 0x0b;
    // p[4] = 0x00
    p[5] = 0x09; // direct mode
    p[6] = 0x64; // brightness LED1
    p[7] = 0x64; // brightness LED2
    p[8] = 0x64; // brightness global
    p[9] = 0x0b; // speed placeholder
    for (i, c) in colors.iter().enumerate().take(2) {
        let b = 10 + i * 3;
        p[b] = c.r;
        p[b + 1] = c.g;
        p[b + 2] = c.b;
    }
    p
}
