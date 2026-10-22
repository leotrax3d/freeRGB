// SPDX-License-Identifier: GPL-2.0-or-later
use freergb_core::Rgb;
pub fn frames(c: Rgb) -> Vec<Vec<u8>> {
    let r = 0x0f - c.r / 16;
    let g = 0x0f - c.g / 16;
    let b = 0x0f - c.b / 16;
    (0..=0x0a)
        .step_by(2)
        .map(|i| vec![0x07, 0x14, (i << 4) | g, (r << 4) | b, 0, 0, 0, 0])
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact() {
        let p = frames(Rgb {
            r: 0,
            g: 255,
            b: 128,
        });
        assert_eq!(p.len(), 6);
        assert_eq!(p[0], [7, 20, 0, 0xf7, 0, 0, 0, 0]);
    }
}
