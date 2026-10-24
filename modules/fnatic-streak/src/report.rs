// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    let mut p = vec![0u8; 64];
    p[0] = 0x0f;
    p[1] = 0x03;
    p[2] = c.r;
    p[3] = c.g;
    p[4] = c.b;
    vec![p]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(&p[0][..5], [15, 3, 1, 2, 3]);
        assert_eq!(p[0].len(), 64);
    }
}
