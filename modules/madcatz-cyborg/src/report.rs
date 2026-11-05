// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET_LEN: usize = 9;
pub fn init() -> [u8; 2] {
    [0xA1, 0x00]
}
pub fn color(c: Rgb) -> [u8; 9] {
    [0xA2, 0x00, c.r, c.g, c.b, 0x00, 0x00, 0x00, 0x00]
}
pub fn intensity(brightness: u8) -> [u8; 3] {
    let b = brightness.min(100);
    [0xA6, 0x00, b]
}
