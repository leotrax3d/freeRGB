// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn activate() -> [u8; 32] {
    let mut b = [0u8; 32];
    b[1] = 0x55; // report id
    b[2] = 0x0F; // command length
    b[3] = 0x01; // LEGO_DIMENSIONS_TOYPAD_BASE_ACTIVATE_VALUE
    b[4] = 0x01;
    // string "(c) LEGO 2014"
    let s = [
        0x28u8, 0x63, 0x29, 0x20, 0x4C, 0x45, 0x47, 0x4F, 0x20, 0x32, 0x30, 0x31, 0x34,
    ];
    for (i, v) in s.iter().enumerate() {
        b[5 + i] = *v;
    }
    b[18] = 0xF7;
    b
}
pub fn direct(zone_idx: usize, c: Rgb) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[1] = 0x55;
    b[2] = 0x06;
    b[3] = 0xC0; // direct mode value
    b[4] = 0x02;
    b[5] = (zone_idx + 1) as u8;
    b[6] = c.r;
    b[7] = c.g;
    b[8] = c.b;
    let checksum = b[1..9].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    b[9] = checksum;
    b
}
