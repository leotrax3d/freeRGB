// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const REPORT_LEN: usize = 64;
pub fn direct(c: Rgb) -> [u8; REPORT_LEN] {
    let mut b = [0u8; REPORT_LEN];
    let command: [u8; 27] = [
        0x1C, 0x00, 0x11, 0x00, 0x01, 0x00, 0x00, 0x00, 0x15, 0x00, 0x28, 0x00, 0x26, 0x00, 0x4C,
        0x00, 0x26, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x80, 0x80, c.r, c.g, c.b,
    ];
    b[0] = 0x21;
    b[1] = 0x12;
    let mut sum: u16 = (b[0] as u16) | ((b[1] as u16) << 8);
    let mut i = 0usize;
    while i < command.len() {
        let mut word = command[i] as u16;
        if i + 1 < command.len() {
            word |= (command[i + 1] as u16) << 8;
        }
        sum = sum.wrapping_add(word);
        i += 2;
    }
    b[2] = (sum & 0xFF) as u8;
    b[3] = (sum >> 8) as u8;
    for (i, v) in command.iter().enumerate() {
        b[4 + i] = *v;
    }
    // total 4 + 27 =31, remaining zero
    b
}
