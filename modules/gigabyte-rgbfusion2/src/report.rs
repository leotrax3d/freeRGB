// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(header: u8, colors: &[Rgb]) -> Vec<u8> {
    let mut p = vec![0; 64];
    p[0] = 0xcc;
    p[1] = header;
    p[2] = (colors.len() * 3) as u8;
    p[3] = 0;
    for (i, c) in colors.iter().take(19).enumerate() {
        let q = 5 + i * 3;
        p[q..q + 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = direct(0x58, &[Rgb { r: 1, g: 2, b: 3 }]);
        assert_eq!(&p[0..9], &[0xcc, 0x58, 3, 0, 0, 1, 2, 3, 0]);
        assert_eq!(p.len(), 64);
    }
}
