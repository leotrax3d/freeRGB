// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;

/// Upstream constants from `RoccatSenseAimoController.h`.
pub const REPORT_SIZE_DIRECT: usize = 9;

/// Upstream: RoccatSenseAimoController.cpp SendDirect: 9-byte feature report
/// `[03] [R G B FF] [R G B FF]`.
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0u8; 9];
    p[0] = 0x03;
    for (i, c) in colors.iter().enumerate().take(2) {
        let b = 1 + i * 4;
        p[b] = c.r;
        p[b + 1] = c.g;
        p[b + 2] = c.b;
        p[b + 3] = 0xff;
    }
    p
}
