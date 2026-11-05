// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
/// upstream: MSIOptixController.cpp SetDirect
pub const REPORT_LEN: usize = 168;
pub fn direct(colors: &[Rgb; 12]) -> [u8; REPORT_LEN] {
    let mut p = [0u8; REPORT_LEN];
    p[0] = 0x72;
    // first header block indices 1..11
    let hdr = [
        0x01u8, 0xFF, 0xFF, 0xFF, 0x00, 0x64, 0x00, 0xFF, 0x00, 0x80, 0x00,
    ];
    for (i, b) in hdr.iter().enumerate() {
        p[1 + i] = *b;
    }
    // duplicated header indices 12..22
    for (i, b) in hdr.iter().enumerate() {
        p[12 + i] = *b;
    }
    // colors start at index 95
    let mut off = 95usize;
    for c in colors.iter() {
        p[off] = c.r;
        p[off + 1] = c.g;
        p[off + 2] = c.b;
        off += 3;
    }
    p
}
