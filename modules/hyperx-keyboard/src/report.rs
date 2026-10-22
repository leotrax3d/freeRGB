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
const KEYS: [usize; 106] = [
    0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x20, 0x21, 0x22, 0x23, 0x24, 0x26, 0x27, 0x28, 0x29,
    0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x37, 0x38, 0x39, 0x3a, 0x3b,
    0x3c, 0x3e, 0x3f, 0x41, 0x44, 0x45, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x51, 0x54,
    0x55, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5e, 0x5f, 0x61, 0x64, 0x65, 0x68, 0x69, 0x6a, 0x6b, 0x6c,
    0x6e, 0x6f, 0x74, 0x75, 0x78, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x81, 0x84, 0x85, 0x88,
    0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x91, 0x94, 0x95,
];
pub fn channel(colors: &[Rgb], number: u8, component: u8) -> Vec<u8> {
    let mut p = vec![0; 264];
    p[0] = 7;
    p[1] = 0x16;
    p[2] = number;
    p[3] = 0xa0;
    for (c, q) in colors.iter().take(106).zip(KEYS) {
        p[q] = match component {
            1 => c.r,
            2 => c.g,
            _ => c.b,
        };
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
