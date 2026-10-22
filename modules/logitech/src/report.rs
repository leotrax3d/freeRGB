// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c: &[Rgb]) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 2;
    for (i, x) in c.iter().take(20).enumerate() {
        p[1 + i * 3..4 + i * 3].copy_from_slice(&[x.r, x.g, x.b]);
    }
    p
}
pub fn g213(zone: usize, c: Rgb) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..10].copy_from_slice(&[
        0x11, 0xff, 0x0c, 0x3a, zone as u8, 0x01, c.r, c.g, c.b, 0x02,
    ]);
    p
}
pub fn g203(colors: &[Rgb]) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..4].copy_from_slice(&[0x11, 0xff, 0x12, 0x10]);
    for (n, c) in colors.iter().take(3).enumerate() {
        let q = 4 + n * 4;
        p[q..q + 4].copy_from_slice(&[(n + 1) as u8, c.r, c.g, c.b]);
    }
    p[16] = 0xff;
    p
}
pub fn g203_apply() -> [u8; 20] {
    let mut p = [0; 20];
    p[0..4].copy_from_slice(&[0x11, 0xff, 0x12, 0x70]);
    p
}
