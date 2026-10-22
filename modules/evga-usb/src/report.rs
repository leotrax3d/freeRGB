// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn mouse(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 65];
    p[1] = 0xa1;
    p[14] = colors.len() as u8;
    for (i, c) in colors.iter().take(3).enumerate() {
        let q = 15 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn x12(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 40];
    p[0] = 7;
    p[1] = 0xea;
    p[2] = 2;
    p[3] = 2;
    for (i, c) in colors.iter().take(3).enumerate() {
        let q = 8 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
pub fn keyboard(colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 136];
    p[0] = 6;
    p[1] = 0xea;
    p[2] = 2;
    p[3] = 1;
    for (i, c) in colors.iter().take(24).enumerate() {
        let q = 8 + i * 5;
        p[q] = 10;
        p[q + 1] = 100;
        p[q + 2..q + 5].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let c = [Rgb { r: 1, g: 2, b: 3 }];
        assert_eq!(
            &mouse(&c)[0..19],
            &[0, 0xa1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 3, 0]
        );
        assert_eq!(&x12(&c)[0..12], &[7, 0xea, 2, 2, 0, 0, 0, 0, 1, 2, 3, 0]);
        assert_eq!(keyboard(&c).len(), 136);
    }
}
