// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    let mut zone = vec![0u8; 64];
    zone[0] = 2;
    zone[1] = 1;
    zone[2] = 0x0f;
    let mut p = vec![0u8; 64];
    p[0] = 2;
    p[2] = 1;
    p[3] = 1;
    p[4] = 0x64;
    p[5] = 0x0a;
    p[6..9].copy_from_slice(&[c.r, c.g, c.b]);
    vec![zone, p]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(p.len(), 2);
        assert_eq!(&p[0][..3], [2, 1, 15]);
        assert_eq!(&p[1][..9], [2, 0, 1, 1, 100, 10, 1, 2, 3]);
    }
}
