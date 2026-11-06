// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET_LEN: usize = 32;
pub fn direct(c: Rgb) -> [u8; PACKET_LEN] {
    let mut b = [0u8; PACKET_LEN];
    let pattern: [(usize, u8); 29] = [
        (0, 0x20),
        (1, 0x03),
        (2, 0x02),
        (3, 0x00),
        (4, 0x01),
        (5, 0x00),
        (9, 0xFF),
        (10, 0x00),
        (11, 0x00),
        (12, 0xFF),
        (13, 0x3F),
        (14, 0x00),
        (15, 0xFF),
        (16, 0xFF),
        (17, 0x00),
        (18, 0x00),
        (19, 0xFF),
        (20, 0x00),
        (21, 0x00),
        (22, 0x00),
        (23, 0xFF),
        (24, 0x00),
        (25, 0xFF),
        (26, 0xFF),
        (27, 0xFF),
        (28, 0x00),
        (29, 0xFF),
        (30, 0x32),
        (31, 0x32),
    ];
    for (i, v) in pattern {
        b[i] = v;
    }
    b[6] = c.r;
    b[7] = c.g;
    b[8] = c.b;
    b
}
