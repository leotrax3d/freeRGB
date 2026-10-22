// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub const PACKET: usize = 64;
pub fn command(sub: u8, property: u8) -> [u8; PACKET] {
    let mut p = [0; PACKET];
    p[1] = sub;
    p[2] = property;
    p
}
pub fn init() -> [[u8; PACKET]; 3] {
    let mut a = [[0; PACKET]; 3];
    a[0] = command(0x08, 0x01);
    a[0][3] = 0x03;
    a[0][5] = 0x02;
    a[1] = command(0x08, 0x02);
    a[1][3] = 0x5f;
    a[2] = command(0x08, 0x0d);
    a[2][4] = 0x01;
    a
}
pub fn mouse(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = command(0x07, 0x0d);
    p[3] = colors.len().min(14) as u8;
    for (n, c) in colors.iter().take(14).enumerate() {
        let q = 5 + n * 4;
        p[q..q + 4].copy_from_slice(&[n as u8, c.r, c.g, c.b]);
    }
    p
}
pub fn strip(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = command(0x07, 0x0d);
    p[3] = colors.len().min(19) as u8;
    for (n, c) in colors.iter().take(19).enumerate() {
        let q = 5 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn keyboard(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = command(0x07, 0x06);
    p[4] = 0x9b;
    p[5] = 1;
    for (n, c) in colors.iter().take(20).enumerate() {
        let q = 12 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn k65_init() -> [[u8; PACKET]; 3] {
    let mut a = [[0; PACKET]; 3];
    a[0] = command(0x08, 1);
    a[0][3] = 3;
    a[0][5] = 2;
    a[1] = command(0x08, 2);
    a[1][3] = 0x6e;
    a[2] = command(0x08, 0x0d);
    a[2][3] = 1;
    a[2][4] = 0x22;
    a
}
pub fn k65(colors: &[Rgb]) -> [u8; PACKET] {
    let mut p = command(0x08, 6);
    p[3] = 1;
    p[4] = 0x73;
    p[5] = 1;
    p[8] = 0x12;
    for (n, c) in colors.iter().take(18).enumerate() {
        let q = 10 + n * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
