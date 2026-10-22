// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn hidpp(feature: u8, function: u8, data: &[u8]) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..4].copy_from_slice(&[0x11, 2, feature, function | 0x07]);
    let n = data.len().min(16);
    p[4..4 + n].copy_from_slice(&data[..n]);
    p
}
pub fn lightspeed_effect(feature: u8, fp8070: bool, zone: u8, c: Rgb) -> [u8; 20] {
    let mut data = [0; 16];
    data[0..5].copy_from_slice(&[zone, 1, c.r, c.g, c.b]);
    data[12] = if fp8070 { 0 } else { 1 };
    hidpp(feature, if fp8070 { 0x30 } else { 0x10 }, &data)
}
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
pub fn g203l_init() -> [u8; 20] {
    let mut p = [0; 20];
    p[0..7].copy_from_slice(&[0x11, 0xff, 0x0e, 0x50, 1, 3, 7]);
    p
}
pub fn g203l(colors: &[Rgb]) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..4].copy_from_slice(&[0x11, 0xff, 0x12, 0x10]);
    for (n, c) in colors.iter().take(3).enumerate() {
        let q = 4 + n * 4;
        p[q..q + 4].copy_from_slice(&[(n + 1) as u8, c.r, c.g, c.b]);
    }
    p[0x10] = 0xff;
    p
}
pub fn g203_apply() -> [u8; 20] {
    let mut p = [0; 20];
    p[0..4].copy_from_slice(&[0x11, 0xff, 0x12, 0x70]);
    p
}
pub fn g815_little(key: u8, c: Rgb) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..8].copy_from_slice(&[0x11, 0xff, 0x10, 0x1f, key, c.r, c.g, c.b]);
    p
}
pub fn g915_little(wired: bool, key: u8, c: Rgb) -> [u8; 20] {
    let mut p = [0; 20];
    p[0] = 0x11;
    p[1] = if wired { 0xff } else { 1 };
    p[2] = if wired { 0x0a } else { 0x0b };
    p[3] = 0x1f;
    p[4..8].copy_from_slice(&[key, c.r, c.g, c.b]);
    p
}
pub fn g815_init() -> [[u8; 20]; 4] {
    let mut a = [[0; 20]; 4];
    a[0][0..4].copy_from_slice(&[0x11, 0xff, 0x08, 0x3e]);
    a[1][0..4].copy_from_slice(&[0x11, 0xff, 0x08, 0x1e]);
    a[2][0..4].copy_from_slice(&[0x11, 0xff, 0x0f, 0x1e]);
    a[2][0x10] = 1;
    a[3][0..5].copy_from_slice(&[0x11, 0xff, 0x0f, 0x1e, 1]);
    a[3][0x10] = 1;
    a
}
pub fn lightsync(function: u8, zone: usize, c: Rgb) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..10].copy_from_slice(&[
        0x11, 1, 0x0e, function, zone as u8, 0x01, c.r, c.g, c.b, 0x02,
    ]);
    p
}
pub fn x56_brightness(brightness: u8) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 9;
    p[2] = 2;
    p[3] = brightness;
    p
}
pub fn x56_color(c: Rgb) -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 9;
    p[2] = 3;
    p[3] = c.r;
    p[4] = c.g;
    p[5] = c.b;
    p
}
pub fn x56_save() -> [u8; 64] {
    let mut p = [0; 64];
    p[0] = 1;
    p[1] = 1;
    p
}
pub fn g600(mode: u8, speed: u16, c: Rgb) -> [u8; 8] {
    [0xf1, c.r, c.g, c.b, mode, speed as u8, 0, 0]
}
pub fn g933(zone: usize, c: Rgb) -> [u8; 20] {
    let mut p = [0; 20];
    p[0..10].copy_from_slice(&[0x11, 0xff, 4, 0x3a, zone as u8, 1, c.r, c.g, c.b, 2]);
    p
}
