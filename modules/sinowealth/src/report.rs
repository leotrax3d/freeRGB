// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[0] = 0x04;
    for (c, i) in colors.iter().take(16).zip(0..) {
        let q = 4 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn mouse(colors: &[Rgb], sequence: u8) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[3] = 2;
    p[4] = sequence;
    p[5] = 1;
    for (c, i) in colors.iter().take(3).zip(0..) {
        let q = 8 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_is_byte_exact() {
        assert_eq!(
            &direct(&[Rgb { r: 1, g: 2, b: 3 }])[..8],
            &[4, 0, 0, 0, 1, 2, 3, 0]
        );
    }
    #[test]
    fn mouse_is_byte_exact() {
        assert_eq!(
            &mouse(&[Rgb { r: 1, g: 2, b: 3 }], 1)[..11],
            &[0, 0, 0, 2, 1, 1, 0, 0, 1, 2, 3]
        );
    }
}
