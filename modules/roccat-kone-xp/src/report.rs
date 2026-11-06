// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream: RoccatKoneXPController basic support.
pub const DIRECT_REPORT_SIZE: usize = 122;

pub fn enter_direct() -> Vec<u8> {
    vec![0x0e, 0x06, 0x01, 0x01, 0x00, 0xff]
}

pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; DIRECT_REPORT_SIZE];
    p[0] = 0x0d;
    p[1] = 0x7a;
    for (i, c) in colors.iter().enumerate().take(20) {
        let b = 2 + i * 6;
        p[b] = c.r;
        p[b + 1] = c.g;
        p[b + 2] = c.b;
        p[b + 3] = 0xff;
        p[b + 4] = 0xff;
        p[b + 5] = 0xff;
    }
    p
}
