// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn init_normal() -> [u8; 65] {
    let mut b = [0u8; 65];
    b[1] = 0x44;
    b[2] = 0x81;
    b[3] = 0x02;
    b
}
pub fn mode_status() -> [u8; 65] {
    let mut b = [0u8; 65];
    b[1] = 0x4C;
    b[2] = 0x81;
    b[3] = 0x07;
    b
}
pub fn color_status(mode: u8) -> [u8; 65] {
    let mut b = [0u8; 65];
    b[1] = 0x4C;
    b[2] = 0x81;
    b[3] = 0x03;
    b[4] = mode;
    b
}
pub fn direct_init() -> [u8; 65] {
    let mut b = [0u8; 65];
    b[1] = 0x5A;
    b[2] = 0x81;
    b[3] = 0x02;
    b
}
pub fn direct_color(c: Rgb) -> [u8; 65] {
    let mut b = [0u8; 65];
    b[1] = 0x5A;
    b[2] = 0x81;
    b[3] = 0x03;
    b[4] = c.r;
    b[5] = c.g;
    b[6] = c.b;
    b
}
