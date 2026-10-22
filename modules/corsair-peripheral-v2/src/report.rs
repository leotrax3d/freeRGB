// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET: usize = 65;
pub fn direct(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 8;
    p[1] = 3;
    p[2] = 1;
    p[3] = 1;
    for (n, c) in colors.iter().take(20).enumerate() {
        let q = 4 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn effect(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[0] = 8;
    p[1] = 3;
    p[2] = 0x22;
    p[3] = 3;
    for (n, c) in colors.iter().take(16).enumerate() {
        let q = 5 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
