// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    vec![
        vec![0xea, 0x08, 0x78, 0x08, 0, c.r, c.g, c.b, c.r],
        vec![0xea, 0x03, 0x78, 0x0a],
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(p[0], [0xea, 8, 0x78, 8, 0, 1, 2, 3, 1]);
        assert_eq!(p[1], [0xea, 3, 0x78, 0x0a]);
    }
}
