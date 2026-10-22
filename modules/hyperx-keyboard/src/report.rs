// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn origins_init(mode: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 4;
    p[2] = 0xf2;
    p[9] = mode;
    p
}
pub fn origins(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 65];
    for (c, i) in colors.iter().take(16).zip(0..) {
        let q = 1 + i * 4;
        p[q] = 0x81;
        p[q + 1] = c.r;
        p[q + 2] = c.g;
        p[q + 3] = c.b;
    }
    p
}
pub fn origins2(seq: u8, colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0x44;
    p[1] = 2;
    p[2] = seq;
    for (c, i) in colors.iter().take(20).zip(0..) {
        let q = 4 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn eve(colors: &[Rgb]) -> Vec<u8> {
    let mut p = origins2(0, colors);
    p[0] = 0x44;
    p
}
pub fn core(colors: &[Rgb], seq: u8) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0xa2;
    p[2] = seq;
    p[4] = (colors.len().min(20) * 3) as u8;
    for (c, i) in colors.iter().take(20).zip(0..) {
        let q = 5 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.g, c.r, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_exact() {
        assert_eq!(
            &origins(&[Rgb { r: 1, g: 2, b: 3 }])[..5],
            &[0, 0x81, 1, 2, 3]
        );
    }
    #[test]
    fn origins2_exact() {
        assert_eq!(
            &origins2(2, &[Rgb { r: 1, g: 2, b: 3 }])[..8],
            &[0x44, 2, 2, 0, 1, 2, 3, 0]
        );
    }
    #[test]
    fn core_exact() {
        assert_eq!(
            &core(&[Rgb { r: 1, g: 2, b: 3 }], 0)[..9],
            &[0, 0xa2, 0, 0, 3, 2, 1, 3, 0]
        );
    }
}
