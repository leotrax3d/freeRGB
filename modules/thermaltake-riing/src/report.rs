// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const BASE_LEN: usize = 65;
pub const QUAD_LEN: usize = 193;
pub const TRIO_LEN: usize = 65;
pub fn init<const N: usize>() -> [u8; N] {
    let mut p = [0; N];
    p[1] = 0xfe;
    p[2] = 0x33;
    p
}
pub fn base(port: usize, colors: &[Rgb]) -> [u8; BASE_LEN] {
    let mut p = [0; BASE_LEN];
    p[1] = 0x32;
    p[2] = 0x52;
    p[3] = (port + 1) as u8;
    p[4] = 0x18;
    for (i, c) in colors.iter().take(20).enumerate() {
        p[5 + i * 3..8 + i * 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}
pub fn quad(port: usize, colors: &[Rgb]) -> [u8; QUAD_LEN] {
    let mut p = [0; QUAD_LEN];
    p[1] = 0x32;
    p[2] = 0x52;
    p[3] = (port + 1) as u8;
    p[4] = 0x24;
    for (i, c) in colors.iter().take(62).enumerate() {
        p[5 + i * 3..8 + i * 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}
pub fn trio(port: usize, chunk: usize, colors: &[Rgb]) -> [u8; TRIO_LEN] {
    let mut p = [0; TRIO_LEN];
    p[1] = 0x32;
    p[2] = 0x52;
    p[3] = (port + 1) as u8;
    p[4] = 0x24;
    p[5] = 3;
    p[6] = (chunk + 1) as u8;
    for (i, c) in colors.iter().take(19).enumerate() {
        p[8 + i * 3..11 + i * 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}
