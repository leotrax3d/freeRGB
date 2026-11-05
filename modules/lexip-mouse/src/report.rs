// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c: Rgb) -> [u8; 64] {
    let mut b = [0u8; 64];
    b[0] = 0x00; // reported 00 24 01... hid_write with leading zero padding? 64 total.
    b[1] = 0x24;
    b[2] = 0x01;
    b[3] = c.r;
    b[4] = c.g;
    b[5] = c.b;
    b[7] = 0x64;
    b[8] = 0x80;
    b
}
