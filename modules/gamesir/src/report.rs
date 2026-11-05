// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c: Rgb) -> [u8; 65] {
    let mut buf = [0u8; 65];
    buf[0] = 0x00;
    buf[1] = 0x05;
    buf[2] = 0x08;
    buf[3] = 0x0A;
    buf[4] = 0x01;
    buf[5] = 0x03;
    buf[6] = c.r;
    buf[7] = c.g;
    buf[8] = c.b;
    buf[9] = 0x00;
    let checksum: u16 = buf[1..=9].iter().map(|&b| b as u16).sum();
    buf[10] = (checksum & 0xFF) as u8;
    buf
}
