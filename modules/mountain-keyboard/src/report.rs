// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    let mut p = vec![0u8; 65];
    p[1] = 0x15;
    p[2] = 0x46;
    p[3] = 0x23;
    p[4] = 0xea;
    p[5] = 25;
    p[6] = 0xc0;
    p[8] = 0x0e;
    p[9] = c.r;
    p[10] = c.g;
    p[11] = c.b;
    vec![p]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(&p[0][1..7], [0x15, 0x46, 0x23, 0xea, 25, 0xc0]);
        assert_eq!(&p[0][9..12], [1, 2, 3]);
    }
}
