// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn direct(c: Rgb) -> Vec<u8> {
    let mut p = vec![0; 17];
    p[0] = 8;
    p[1] = 7;
    p[4] = 0xa0;
    p[5] = 7;
    p[6] = 0x20;
    p[7] = c.r;
    p[8] = c.g;
    p[9] = c.b;
    p[10] = 2;
    p[11] = 0xff;
    p[12] = 0x55u8
        .wrapping_sub(p[6])
        .wrapping_sub(p[7])
        .wrapping_sub(p[8])
        .wrapping_sub(p[9])
        .wrapping_sub(p[10])
        .wrapping_sub(p[11]);
    p[16] = 0x4a;
    p
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        assert_eq!(
            &direct(Rgb { r: 1, g: 2, b: 3 })[0..13],
            &[8, 7, 0, 0, 0xa0, 7, 0x20, 1, 2, 3, 2, 0xff, 0x2e]
        );
    }
}
