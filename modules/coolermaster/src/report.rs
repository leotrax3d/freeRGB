// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET: usize = 64;
pub fn direct(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 1;
    for (n, c) in colors.iter().take(20).enumerate() {
        let q = 1 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn keyboard(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 0x07;
    p[1] = 0x06;
    p[2] = 0x01;
    p[3] = 0x01;
    for (n, c) in colors.iter().take(20).enumerate() {
        let q = 4 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn mouse(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 0x07;
    p[1] = 0x05;
    p[2] = colors.len().min(8) as u8;
    for (n, c) in colors.iter().take(8).enumerate() {
        let q = 4 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
